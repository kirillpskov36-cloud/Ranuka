//! Invariant tests over long random trade sequences (architecture doc, section 8).
//!
//! Uses a small deterministic generator instead of an external crate so that
//! `ranuka-core` stays dependency-free and every failure is reproducible
//! from its seed.

use ranuka_core::constants::{DEFAULT_FEE_BPS, LAMPORTS_PER_SOL, MAX_FEE_BPS, MAX_LAUNCH_FEE_BPS};
use ranuka_core::{
    quote_buy_exact_in, quote_buy_exact_out, quote_sell, CoreError, CurveParams, Reserves,
};

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_1EB5);
        z ^ (z >> 31)
    }

    /// Uniform in `1..=max`.
    fn range(&mut self, max: u64) -> u64 {
        1 + self.next() % max
    }

    /// Spread over orders of magnitude: tiny, small, medium and large amounts.
    fn amount(&mut self, max: u64) -> u64 {
        let cap = match self.next() % 4 {
            0 => 1_000.min(max),
            1 => (max / 10_000).max(1),
            2 => (max / 100).max(1),
            _ => max,
        };
        self.range(cap)
    }
}

const PARAMS: CurveParams = CurveParams::DEFAULT;

/// Checks everything that must hold after any single trade.
fn check_step(before: &Reserves, after: &Reserves) {
    // The constant product never decreases: rounding only ever favors the pool.
    assert!(after.k() >= before.k(), "k decreased");

    // Virtual and real reserves move together.
    assert_eq!(
        after.virtual_token - after.real_token,
        PARAMS.initial_virtual_token - PARAMS.initial_real_token
    );
    assert_eq!(
        after.virtual_sol - after.real_sol,
        PARAMS.initial_virtual_sol
    );

    assert!(after.real_token <= PARAMS.initial_real_token);
}

/// Runs a random mix of trades and returns the final reserves.
/// Buys and sells take separate fees: only buys can carry the launch fee.
fn run_sequence(seed: u64, steps: usize, buy_fee_bps: u16, sell_fee_bps: u16) -> Reserves {
    let mut rng = Rng(seed);
    let mut r = Reserves::new(&PARAMS);

    for _ in 0..steps {
        if r.is_complete() {
            break;
        }
        let sold = PARAMS.initial_real_token - r.real_token;
        let before = r;

        let result = match rng.next() % 3 {
            0 => {
                let budget = rng.amount(5 * LAMPORTS_PER_SOL);
                quote_buy_exact_in(&r, budget, buy_fee_bps).map(|q| {
                    assert!(q.buy_cost().unwrap() <= budget, "paid more than budget");
                    r.apply_buy(&q).unwrap()
                })
            }
            1 => {
                let tokens = rng.amount(r.real_token);
                quote_buy_exact_out(&r, tokens, buy_fee_bps).map(|q| {
                    assert_eq!(q.tokens, tokens);
                    r.apply_buy(&q).unwrap()
                })
            }
            // Only tokens that left the curve can come back.
            _ if sold > 0 => {
                let tokens = rng.amount(sold);
                quote_sell(&r, tokens, sell_fee_bps).map(|q| r.apply_sell(&q).unwrap())
            }
            _ => continue,
        };

        match result {
            Ok(after) => {
                check_step(&before, &after);
                r = after;
            }
            // Dust trades are refused, never mispriced.
            Err(CoreError::ZeroAmount) => {}
            Err(e) => panic!("seed {seed}: unexpected error {e:?}"),
        }
    }
    r
}

#[test]
fn invariants_hold_over_random_sequences() {
    for seed in 0..200 {
        for (buy_fee, sell_fee) in [
            (0, 0),
            (DEFAULT_FEE_BPS, DEFAULT_FEE_BPS),
            (MAX_FEE_BPS, MAX_FEE_BPS),
            // Launch window: elevated buy fee, base sell fee.
            (MAX_LAUNCH_FEE_BPS, DEFAULT_FEE_BPS),
        ] {
            run_sequence(seed, 500, buy_fee, sell_fee);
        }
    }
}

/// Solvency: whatever happened before, the pool can always buy back every
/// token that is outside it, and nothing is left owed afterwards.
#[test]
fn pool_can_always_buy_back_everything() {
    for seed in 0..200 {
        let r = run_sequence(seed, 300, DEFAULT_FEE_BPS, DEFAULT_FEE_BPS);
        if r.is_complete() {
            continue;
        }
        let sold = PARAMS.initial_real_token - r.real_token;
        if sold == 0 {
            continue;
        }

        match quote_sell(&r, sold, DEFAULT_FEE_BPS) {
            Ok(q) => {
                let end = r.apply_sell(&q).unwrap();
                assert_eq!(end.real_token, PARAMS.initial_real_token);
                // Rounding dust stays in the pool; it never goes negative.
                assert!(end.virtual_sol >= PARAMS.initial_virtual_sol);
            }
            Err(CoreError::ZeroAmount) => {}
            Err(e) => panic!("seed {seed}: pool could not buy back: {e:?}"),
        }
    }
}

/// A round trip never profits, at any size and even with a zero fee.
#[test]
fn round_trip_never_profits() {
    let mut rng = Rng(42);
    for _ in 0..20_000 {
        // Start from a random point on the curve.
        let mut r = Reserves::new(&PARAMS);
        let warmup = rng.amount(60 * LAMPORTS_PER_SOL);
        if let Ok(q) = quote_buy_exact_in(&r, warmup, 0) {
            r = r.apply_buy(&q).unwrap();
        }
        if r.is_complete() {
            continue;
        }

        for fee_bps in [0, DEFAULT_FEE_BPS] {
            let budget = rng.amount(10 * LAMPORTS_PER_SOL);
            let Ok(buy) = quote_buy_exact_in(&r, budget, fee_bps) else {
                continue;
            };
            let after_buy = r.apply_buy(&buy).unwrap();
            if after_buy.is_complete() {
                continue;
            }
            match quote_sell(&after_buy, buy.tokens, fee_bps) {
                Ok(sell) => {
                    assert!(sell.sell_proceeds().unwrap() <= buy.buy_cost().unwrap());
                }
                Err(CoreError::ZeroAmount) => {}
                Err(e) => panic!("unexpected error {e:?}"),
            }
        }
    }
}

/// Filling the curve with many small buys lands on the same ~85.005 SOL
/// as a single buy; the difference is rounding dust in the pool's favor.
#[test]
fn many_small_buys_fill_the_curve_at_about_85_sol() {
    for seed in 0..20 {
        let mut rng = Rng(seed);
        let mut r = Reserves::new(&PARAMS);
        let mut trades = 0u32;

        while !r.is_complete() {
            let budget = LAMPORTS_PER_SOL / 100 + rng.range(LAMPORTS_PER_SOL / 2);
            let q = quote_buy_exact_in(&r, budget, DEFAULT_FEE_BPS).unwrap();
            let after = r.apply_buy(&q).unwrap();
            check_step(&r, &after);
            r = after;
            trades += 1;
            assert!(trades < 100_000, "curve never completed");
        }

        assert_eq!(r.real_token, 0);
        assert!(r.real_sol >= 85_005_359_057);
        assert!(r.real_sol < 85_006_000_000);
    }
}
