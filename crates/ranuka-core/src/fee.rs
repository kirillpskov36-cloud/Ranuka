//! Fee schedule: a flat base fee plus an elevated buy fee right after launch.
//!
//! The launch fee starts at `launch_fee_bps` in the creation slot and decays
//! linearly to `base_fee_bps` over `launch_window_slots`. It applies to buys
//! only. Because the fee is a pure function of the slot, every bot can compute
//! it exactly: sniping becomes a public trade-off between entering early at a
//! higher fee and entering later at a higher price, not a latency race.

use crate::constants::{
    DEFAULT_FEE_BPS, DEFAULT_LAUNCH_FEE_BPS, DEFAULT_LAUNCH_WINDOW_SLOTS, MAX_FEE_BPS,
    MAX_LAUNCH_FEE_BPS, MAX_LAUNCH_WINDOW_SLOTS,
};
use crate::error::CoreError;
use crate::math::mul_div_ceil;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeSchedule {
    /// Fee on every sell, and on buys after the launch window.
    pub base_fee_bps: u16,
    /// Buy fee in the creation slot.
    pub launch_fee_bps: u16,
    /// Slots over which the buy fee decays to the base fee. Zero disables
    /// the launch fee.
    pub launch_window_slots: u64,
}

impl FeeSchedule {
    pub const DEFAULT: Self = Self {
        base_fee_bps: DEFAULT_FEE_BPS,
        launch_fee_bps: DEFAULT_LAUNCH_FEE_BPS,
        launch_window_slots: DEFAULT_LAUNCH_WINDOW_SLOTS,
    };

    /// Checks the hard caps. The program runs this on every config change,
    /// so no admin can set a schedule outside them.
    pub const fn validate(&self) -> Result<(), CoreError> {
        if self.base_fee_bps > MAX_FEE_BPS
            || self.launch_fee_bps > MAX_LAUNCH_FEE_BPS
            || self.launch_window_slots > MAX_LAUNCH_WINDOW_SLOTS
            || self.launch_fee_bps < self.base_fee_bps
        {
            return Err(CoreError::InvalidFeeSchedule);
        }
        Ok(())
    }

    /// Fee for a sell. Sells never pay the launch fee.
    pub const fn sell_fee_bps(&self) -> u16 {
        self.base_fee_bps
    }

    /// Fee for a buy made `slots_since_creation` slots after the curve was
    /// created. Rounded up, so the fee is never below the exact linear value.
    pub fn buy_fee_bps(&self, slots_since_creation: u64) -> Result<u16, CoreError> {
        self.validate()?;
        if slots_since_creation >= self.launch_window_slots {
            return Ok(self.base_fee_bps);
        }

        // `validate` guarantees launch >= base and the branch above guarantees
        // window > elapsed, so neither subtraction can fail.
        let extra = self
            .launch_fee_bps
            .checked_sub(self.base_fee_bps)
            .ok_or(CoreError::InvalidFeeSchedule)?;
        let remaining = self
            .launch_window_slots
            .checked_sub(slots_since_creation)
            .ok_or(CoreError::InvalidFeeSchedule)?;

        let decayed = mul_div_ceil(
            u64::from(extra),
            remaining,
            u128::from(self.launch_window_slots),
        )?;
        let fee = u64::from(self.base_fee_bps)
            .checked_add(decayed)
            .ok_or(CoreError::MathOverflow)?;
        u16::try_from(fee).map_err(|_| CoreError::MathOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: FeeSchedule = FeeSchedule::DEFAULT;

    #[test]
    fn default_schedule_is_valid() {
        assert_eq!(S.validate(), Ok(()));
        assert_eq!(S.sell_fee_bps(), 30);
    }

    #[test]
    fn starts_at_launch_fee_and_ends_at_base_fee() {
        assert_eq!(S.buy_fee_bps(0), Ok(5_000));
        assert_eq!(S.buy_fee_bps(150), Ok(30));
        assert_eq!(S.buy_fee_bps(151), Ok(30));
        assert_eq!(S.buy_fee_bps(u64::MAX), Ok(30));
    }

    #[test]
    fn midpoint_matches_reference() {
        // 30 + ceil(4970 * 75 / 150) = 2515
        assert_eq!(S.buy_fee_bps(75), Ok(2_515));
        // 30 + ceil(4970 * 1 / 150) = 30 + 34 = 64
        assert_eq!(S.buy_fee_bps(149), Ok(64));
    }

    #[test]
    fn decays_monotonically_and_stays_within_bounds() {
        let mut prev = S.buy_fee_bps(0).unwrap();
        for slot in 1..=200 {
            let fee = S.buy_fee_bps(slot).unwrap();
            assert!(fee <= prev, "fee rose at slot {slot}");
            assert!(fee >= S.base_fee_bps);
            assert!(fee <= S.launch_fee_bps);
            prev = fee;
        }
    }

    #[test]
    fn zero_window_disables_launch_fee() {
        let s = FeeSchedule {
            launch_window_slots: 0,
            ..S
        };
        assert_eq!(s.validate(), Ok(()));
        assert_eq!(s.buy_fee_bps(0), Ok(30));
    }

    #[test]
    fn hard_caps_are_enforced() {
        let over_base = FeeSchedule {
            base_fee_bps: MAX_FEE_BPS + 1,
            ..S
        };
        let over_launch = FeeSchedule {
            launch_fee_bps: MAX_LAUNCH_FEE_BPS + 1,
            ..S
        };
        let over_window = FeeSchedule {
            launch_window_slots: MAX_LAUNCH_WINDOW_SLOTS + 1,
            ..S
        };
        let launch_below_base = FeeSchedule {
            base_fee_bps: 100,
            launch_fee_bps: 50,
            ..S
        };
        for bad in [over_base, over_launch, over_window, launch_below_base] {
            assert_eq!(bad.validate(), Err(CoreError::InvalidFeeSchedule));
            assert_eq!(bad.buy_fee_bps(0), Err(CoreError::InvalidFeeSchedule));
        }
    }

    #[test]
    fn every_valid_schedule_yields_a_quotable_fee() {
        let max = FeeSchedule {
            base_fee_bps: MAX_FEE_BPS,
            launch_fee_bps: MAX_LAUNCH_FEE_BPS,
            launch_window_slots: MAX_LAUNCH_WINDOW_SLOTS,
        };
        for slot in 0..=MAX_LAUNCH_WINDOW_SLOTS + 1 {
            let fee = max.buy_fee_bps(slot).unwrap();
            assert!(crate::fee_for(1_000_000, fee).is_ok());
        }
    }
}
