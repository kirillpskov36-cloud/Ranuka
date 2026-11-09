//! Pricing: fees, buy and sell quotes, and applying a quote to the reserves.
//!
//! Conventions (spec section 3):
//! - `S` is the SOL that moves through the curve; the fee is charged on top
//!   of it for buys and taken out of it for sells;
//! - whatever the pool gives out is rounded down, whatever it takes in is
//!   rounded up.

use crate::constants::{BPS_DENOMINATOR, MAX_FEE_BPS};
use crate::error::CoreError;
use crate::math::{mul_div_ceil, mul_div_floor};
use crate::types::{Quote, Reserves};

fn check_fee(fee_bps: u16) -> Result<(), CoreError> {
    if fee_bps > MAX_FEE_BPS {
        return Err(CoreError::FeeTooHigh);
    }
    Ok(())
}

/// Fee on `sol` lamports moving through the curve: `ceil(sol * fee_bps / 10_000)`.
pub fn fee_for(sol: u64, fee_bps: u16) -> Result<u64, CoreError> {
    check_fee(fee_bps)?;
    mul_div_ceil(sol, u64::from(fee_bps), u128::from(BPS_DENOMINATOR))
}

/// Tokens the pool gives for `sol` lamports: `floor(vt * sol / (vs + sol))`.
fn tokens_for_sol(r: &Reserves, sol: u64) -> Result<u64, CoreError> {
    let denom = u128::from(r.virtual_sol)
        .checked_add(u128::from(sol))
        .ok_or(CoreError::MathOverflow)?;
    mul_div_floor(r.virtual_token, sol, denom)
}

/// SOL the pool takes for exactly `tokens`: `ceil(vs * tokens / (vt - tokens))`.
fn sol_for_tokens_out(r: &Reserves, tokens: u64) -> Result<u64, CoreError> {
    let denom = r
        .virtual_token
        .checked_sub(tokens)
        .filter(|d| *d > 0)
        .ok_or(CoreError::InsufficientLiquidity)?;
    mul_div_ceil(r.virtual_sol, tokens, u128::from(denom))
}

/// SOL the pool gives for `tokens` sold back: `floor(vs * tokens / (vt + tokens))`.
fn sol_for_tokens_in(r: &Reserves, tokens: u64) -> Result<u64, CoreError> {
    let denom = u128::from(r.virtual_token)
        .checked_add(u128::from(tokens))
        .ok_or(CoreError::MathOverflow)?;
    mul_div_floor(r.virtual_sol, tokens, denom)
}

/// Buy with a total budget of `sol_in` lamports, fee included.
///
/// The buyer never pays more than `sol_in`; rounding dust is not charged.
/// If the budget would buy more than the curve has left, the trade is clipped
/// to the remaining tokens and the cost shrinks accordingly; applying that
/// quote completes the curve.
pub fn quote_buy_exact_in(r: &Reserves, sol_in: u64, fee_bps: u16) -> Result<Quote, CoreError> {
    check_fee(fee_bps)?;
    if r.is_complete() {
        return Err(CoreError::CurveComplete);
    }
    if sol_in == 0 {
        return Err(CoreError::ZeroAmount);
    }

    // Largest S with S + fee(S) <= sol_in.
    let gross = u128::from(BPS_DENOMINATOR)
        .checked_add(u128::from(fee_bps))
        .ok_or(CoreError::MathOverflow)?;
    let mut sol = mul_div_floor(sol_in, BPS_DENOMINATOR, gross)?;
    if sol == 0 {
        return Err(CoreError::ZeroAmount);
    }

    let mut tokens = tokens_for_sol(r, sol)?;
    if tokens > r.real_token {
        tokens = r.real_token;
        sol = sol_for_tokens_out(r, tokens)?;
    }
    if tokens == 0 {
        return Err(CoreError::ZeroAmount);
    }

    let fee = fee_for(sol, fee_bps)?;
    Ok(Quote { sol, tokens, fee })
}

/// Buy exactly `tokens_out` tokens. Never clips: asking for more than the
/// curve has left is `InsufficientLiquidity`.
pub fn quote_buy_exact_out(
    r: &Reserves,
    tokens_out: u64,
    fee_bps: u16,
) -> Result<Quote, CoreError> {
    check_fee(fee_bps)?;
    if r.is_complete() {
        return Err(CoreError::CurveComplete);
    }
    if tokens_out == 0 {
        return Err(CoreError::ZeroAmount);
    }
    if tokens_out > r.real_token {
        return Err(CoreError::InsufficientLiquidity);
    }

    let sol = sol_for_tokens_out(r, tokens_out)?;
    let fee = fee_for(sol, fee_bps)?;
    Ok(Quote {
        sol,
        tokens: tokens_out,
        fee,
    })
}

/// Sell `tokens_in` tokens back to the curve.
///
/// A sale whose proceeds after the fee would be zero is `ZeroAmount`.
pub fn quote_sell(r: &Reserves, tokens_in: u64, fee_bps: u16) -> Result<Quote, CoreError> {
    check_fee(fee_bps)?;
    if r.is_complete() {
        return Err(CoreError::CurveComplete);
    }
    if tokens_in == 0 {
        return Err(CoreError::ZeroAmount);
    }

    let sol = sol_for_tokens_in(r, tokens_in)?;
    if sol > r.real_sol {
        return Err(CoreError::InsufficientLiquidity);
    }
    let fee = fee_for(sol, fee_bps)?;
    if sol <= fee {
        return Err(CoreError::ZeroAmount);
    }
    Ok(Quote {
        sol,
        tokens: tokens_in,
        fee,
    })
}

impl Quote {
    /// What a buyer pays in total: `sol + fee`.
    pub fn buy_cost(&self) -> Result<u64, CoreError> {
        self.sol
            .checked_add(self.fee)
            .ok_or(CoreError::MathOverflow)
    }

    /// What a seller receives: `sol - fee`.
    pub fn sell_proceeds(&self) -> Result<u64, CoreError> {
        self.sol
            .checked_sub(self.fee)
            .ok_or(CoreError::MathOverflow)
    }
}

impl Reserves {
    /// Reserves after a buy. The fee is not part of the reserves.
    pub fn apply_buy(&self, q: &Quote) -> Result<Self, CoreError> {
        Ok(Self {
            virtual_sol: self
                .virtual_sol
                .checked_add(q.sol)
                .ok_or(CoreError::MathOverflow)?,
            virtual_token: self
                .virtual_token
                .checked_sub(q.tokens)
                .ok_or(CoreError::InsufficientLiquidity)?,
            real_sol: self
                .real_sol
                .checked_add(q.sol)
                .ok_or(CoreError::MathOverflow)?,
            real_token: self
                .real_token
                .checked_sub(q.tokens)
                .ok_or(CoreError::InsufficientLiquidity)?,
        })
    }

    /// Reserves after a sell. The fee is not part of the reserves.
    pub fn apply_sell(&self, q: &Quote) -> Result<Self, CoreError> {
        Ok(Self {
            virtual_sol: self
                .virtual_sol
                .checked_sub(q.sol)
                .ok_or(CoreError::InsufficientLiquidity)?,
            virtual_token: self
                .virtual_token
                .checked_add(q.tokens)
                .ok_or(CoreError::MathOverflow)?,
            real_sol: self
                .real_sol
                .checked_sub(q.sol)
                .ok_or(CoreError::InsufficientLiquidity)?,
            real_token: self
                .real_token
                .checked_add(q.tokens)
                .ok_or(CoreError::MathOverflow)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DEFAULT_FEE_BPS, LAMPORTS_PER_SOL, TOKEN_UNIT};
    use crate::types::CurveParams;

    const FEE: u16 = DEFAULT_FEE_BPS;

    fn fresh() -> Reserves {
        Reserves::new(&CurveParams::DEFAULT)
    }

    // Reference values below were computed independently of this code.

    #[test]
    fn fee_rounds_up_and_respects_cap() {
        assert_eq!(fee_for(LAMPORTS_PER_SOL, 30), Ok(3_000_000));
        assert_eq!(fee_for(1, 30), Ok(1));
        assert_eq!(fee_for(0, 30), Ok(0));
        assert_eq!(fee_for(LAMPORTS_PER_SOL, 0), Ok(0));
        assert_eq!(fee_for(LAMPORTS_PER_SOL, MAX_FEE_BPS), Ok(10_000_000));
        assert_eq!(
            fee_for(LAMPORTS_PER_SOL, MAX_FEE_BPS + 1),
            Err(CoreError::FeeTooHigh)
        );
    }

    #[test]
    fn first_buy_of_one_sol_matches_reference() {
        let q = quote_buy_exact_in(&fresh(), LAMPORTS_PER_SOL, FEE).unwrap();
        assert_eq!(q.sol, 997_008_973);
        assert_eq!(q.fee, 2_991_027);
        assert_eq!(q.tokens, 34_512_705_047_149);
        assert_eq!(q.buy_cost(), Ok(LAMPORTS_PER_SOL));
    }

    #[test]
    fn buy_then_sell_matches_reference_and_loses_money() {
        let r0 = fresh();
        let buy = quote_buy_exact_in(&r0, LAMPORTS_PER_SOL, FEE).unwrap();
        let r1 = r0.apply_buy(&buy).unwrap();

        let sell = quote_sell(&r1, buy.tokens, FEE).unwrap();
        assert_eq!(sell.sol, 997_008_972);
        assert_eq!(sell.fee, 2_991_027);
        assert_eq!(sell.sell_proceeds(), Ok(994_017_945));
        assert!(sell.sell_proceeds().unwrap() < buy.buy_cost().unwrap());

        // The lamport lost to rounding stays in the pool.
        let r2 = r1.apply_sell(&sell).unwrap();
        assert_eq!(r2.real_token, r0.real_token);
        assert_eq!(r2.real_sol, 1);
    }

    #[test]
    fn exact_out_matches_reference() {
        let q = quote_buy_exact_out(&fresh(), 1_000_000 * TOKEN_UNIT, FEE).unwrap();
        assert_eq!(q.tokens, 1_000_000 * TOKEN_UNIT);
        assert_eq!(q.sol, 27_985_075);
        assert_eq!(q.fee, 83_956);
    }

    #[test]
    fn exact_in_with_exact_out_cost_buys_at_least_as_much() {
        let r = fresh();
        for tokens in [
            1,
            7,
            TOKEN_UNIT,
            123_456 * TOKEN_UNIT,
            500_000_000 * TOKEN_UNIT,
        ] {
            let out = quote_buy_exact_out(&r, tokens, FEE).unwrap();
            let back = quote_buy_exact_in(&r, out.buy_cost().unwrap(), FEE).unwrap();
            assert!(back.tokens >= tokens);
            assert!(back.buy_cost().unwrap() <= out.buy_cost().unwrap());
        }
    }

    #[test]
    fn oversized_buy_is_clipped_and_completes_the_curve() {
        let r0 = fresh();
        let budget = 1_000 * LAMPORTS_PER_SOL;
        let q = quote_buy_exact_in(&r0, budget, FEE).unwrap();

        assert_eq!(q.tokens, r0.real_token);
        assert_eq!(q.sol, 85_005_359_057);
        assert_eq!(q.fee, 255_016_078);
        assert!(q.buy_cost().unwrap() < budget);

        let r1 = r0.apply_buy(&q).unwrap();
        assert!(r1.is_complete());
        assert_eq!(r1.real_sol, 85_005_359_057);
    }

    #[test]
    fn complete_curve_rejects_all_trades() {
        let r0 = fresh();
        let q = quote_buy_exact_in(&r0, 1_000 * LAMPORTS_PER_SOL, FEE).unwrap();
        let done = r0.apply_buy(&q).unwrap();

        assert_eq!(
            quote_buy_exact_in(&done, LAMPORTS_PER_SOL, FEE),
            Err(CoreError::CurveComplete)
        );
        assert_eq!(
            quote_buy_exact_out(&done, 1, FEE),
            Err(CoreError::CurveComplete)
        );
        assert_eq!(quote_sell(&done, 1, FEE), Err(CoreError::CurveComplete));
    }

    #[test]
    fn rejects_zero_and_dust_amounts() {
        let r = fresh();
        assert_eq!(quote_buy_exact_in(&r, 0, FEE), Err(CoreError::ZeroAmount));
        // 1 lamport cannot cover even a 1-lamport trade plus its fee.
        assert_eq!(quote_buy_exact_in(&r, 1, FEE), Err(CoreError::ZeroAmount));
        assert_eq!(quote_buy_exact_out(&r, 0, FEE), Err(CoreError::ZeroAmount));
        assert_eq!(quote_sell(&r, 0, FEE), Err(CoreError::ZeroAmount));
    }

    #[test]
    fn exact_out_beyond_remaining_tokens_is_rejected() {
        let r = fresh();
        assert_eq!(
            quote_buy_exact_out(&r, r.real_token + 1, FEE),
            Err(CoreError::InsufficientLiquidity)
        );
        // Exactly the remainder is fine.
        assert!(quote_buy_exact_out(&r, r.real_token, FEE).is_ok());
    }

    #[test]
    fn cannot_sell_more_than_the_pool_can_pay() {
        // Nothing was bought, so the pool holds no real SOL to pay out.
        assert_eq!(
            quote_sell(&fresh(), 1_000_000 * TOKEN_UNIT, FEE),
            Err(CoreError::InsufficientLiquidity)
        );
    }

    #[test]
    fn fee_above_cap_is_rejected_everywhere() {
        let r = fresh();
        let bad = MAX_FEE_BPS + 1;
        assert_eq!(
            quote_buy_exact_in(&r, LAMPORTS_PER_SOL, bad),
            Err(CoreError::FeeTooHigh)
        );
        assert_eq!(
            quote_buy_exact_out(&r, TOKEN_UNIT, bad),
            Err(CoreError::FeeTooHigh)
        );
        assert_eq!(quote_sell(&r, TOKEN_UNIT, bad), Err(CoreError::FeeTooHigh));
    }

    #[test]
    fn extreme_inputs_return_errors_instead_of_panicking() {
        let r = fresh();
        let _ = quote_buy_exact_in(&r, u64::MAX, FEE);
        let _ = quote_buy_exact_out(&r, u64::MAX, FEE);
        let _ = quote_sell(&r, u64::MAX, FEE);

        let weird = Reserves {
            virtual_sol: u64::MAX,
            virtual_token: u64::MAX,
            real_sol: u64::MAX,
            real_token: u64::MAX,
        };
        let _ = quote_buy_exact_in(&weird, u64::MAX, FEE);
        let _ = quote_buy_exact_out(&weird, u64::MAX, FEE);
        let _ = quote_sell(&weird, u64::MAX, FEE);
    }
}
