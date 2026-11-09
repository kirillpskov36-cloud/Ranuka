use anchor_lang::prelude::*;
use ranuka_core::FeeSchedule;

/// Platform-wide config. One per program, at the PDA `["global"]`.
///
/// Every field has a fixed size, so field offsets never move; that is what
/// lets clients read single fields straight from the account bytes.
/// Trades only ever read this account.
#[account]
#[derive(InitSpace)]
pub struct Global {
    pub version: u8,
    pub bump: u8,
    pub admin: Pubkey,
    /// Default address when no transfer is pending.
    pub pending_admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub base_fee_bps: u16,
    pub launch_fee_bps: u16,
    pub launch_window_slots: u64,
    /// Parameters every new curve starts with. Fixed at initialization.
    pub initial_virtual_sol: u64,
    pub initial_virtual_token: u64,
    pub initial_real_token: u64,
    pub total_supply: u64,
    /// Room for future fields without resizing the account.
    pub reserved: [u8; 64],
}

impl Global {
    /// Account size including Anchor's 8-byte discriminator.
    pub const SPACE: usize = 8 + Self::INIT_SPACE;

    pub fn fee_schedule(&self) -> FeeSchedule {
        FeeSchedule {
            base_fee_bps: self.base_fee_bps,
            launch_fee_bps: self.launch_fee_bps,
            launch_window_slots: self.launch_window_slots,
        }
    }

    pub fn set_fee_schedule(&mut self, schedule: &FeeSchedule) {
        self.base_fee_bps = schedule.base_fee_bps;
        self.launch_fee_bps = schedule.launch_fee_bps;
        self.launch_window_slots = schedule.launch_window_slots;
    }
}
