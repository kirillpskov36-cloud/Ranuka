//! Curve data types: launch parameters, live reserves, quote result.

use crate::constants::{LAMPORTS_PER_SOL, TOKEN_UNIT};

/// Parameters every curve is created with. Identical for all tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurveParams {
    /// Initial virtual SOL reserve, lamports.
    pub initial_virtual_sol: u64,
    /// Initial virtual token reserve, base units.
    pub initial_virtual_token: u64,
    /// Tokens sold on the curve.
    pub initial_real_token: u64,
    /// Total token supply.
    pub total_supply: u64,
}

impl CurveParams {
    /// Ranuka launch parameters (spec section 3).
    pub const DEFAULT: Self = Self {
        initial_virtual_sol: 30 * LAMPORTS_PER_SOL,
        initial_virtual_token: 1_073_000_000 * TOKEN_UNIT,
        initial_real_token: 793_100_000 * TOKEN_UNIT,
        total_supply: 1_000_000_000 * TOKEN_UNIT,
    };

    /// Tokens held back for migration. `None` if the parameters are inconsistent.
    pub const fn migration_reserve(&self) -> Option<u64> {
        self.total_supply.checked_sub(self.initial_real_token)
    }
}

/// Live curve state; changes with every trade.
///
/// Price depends on the virtual reserves only (`virtual_sol / virtual_token`).
/// Real reserves track what the vaults actually hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reserves {
    /// Initial virtual SOL plus everything buyers paid in, lamports.
    pub virtual_sol: u64,
    /// Virtual token reserve; shrinks on buys.
    pub virtual_token: u64,
    /// SOL actually held by the curve, excluding fees.
    pub real_sol: u64,
    /// Tokens still purchasable on the curve.
    pub real_token: u64,
}

impl Reserves {
    /// State of a freshly created curve.
    pub const fn new(params: &CurveParams) -> Self {
        Self {
            virtual_sol: params.initial_virtual_sol,
            virtual_token: params.initial_virtual_token,
            real_sol: 0,
            real_token: params.initial_real_token,
        }
    }

    /// A curve is complete once every token offered on it is sold.
    pub const fn is_complete(&self) -> bool {
        self.real_token == 0
    }

    /// Curve invariant `k = virtual_sol * virtual_token`.
    // The product of two u64 values always fits in u128.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn k(&self) -> u128 {
        u128::from(self.virtual_sol) * u128::from(self.virtual_token)
    }
}

/// Result of pricing a trade. Pricing never mutates state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    /// SOL moving through the curve, lamports, fee excluded:
    /// a buyer pays `sol + fee`, a seller receives `sol - fee`.
    pub sol: u64,
    /// Tokens the buyer receives or the seller gives up.
    pub tokens: u64,
    /// Fee, lamports.
    pub fee: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params_are_consistent() {
        let p = CurveParams::DEFAULT;
        assert_eq!(p.migration_reserve(), Some(206_900_000 * TOKEN_UNIT));
        // Otherwise the price of the last token would be unbounded.
        assert!(p.initial_virtual_token > p.initial_real_token);
    }

    #[test]
    fn migration_reserve_rejects_bad_params() {
        let bad = CurveParams {
            initial_real_token: 2 * TOKEN_UNIT,
            total_supply: TOKEN_UNIT,
            ..CurveParams::DEFAULT
        };
        assert_eq!(bad.migration_reserve(), None);
    }

    #[test]
    fn new_reserves_start_empty() {
        let r = Reserves::new(&CurveParams::DEFAULT);
        assert_eq!(r.real_sol, 0);
        assert_eq!(r.virtual_sol, 30 * LAMPORTS_PER_SOL);
        assert_eq!(r.real_token, 793_100_000 * TOKEN_UNIT);
        assert!(!r.is_complete());
    }

    /// Spec check: selling out the curve leaves about 85.005 SOL in it.
    #[test]
    fn full_curve_holds_about_85_sol() {
        let p = CurveParams::DEFAULT;
        let start = Reserves::new(&p);

        let final_virtual_token = p.initial_virtual_token - p.initial_real_token;
        let final_virtual_sol = start.k() / u128::from(final_virtual_token);
        let real_sol = final_virtual_sol - u128::from(p.initial_virtual_sol);

        assert!(real_sol >= 85_005_000_000);
        assert!(real_sol < 85_006_000_000);
    }
}
