//! Events are emitted through a self-CPI (`emit_cpi!`), so they survive log
//! truncation and can be parsed from the transaction's inner instructions.

use anchor_lang::prelude::*;

/// The global config was created or changed. Carries the full new config.
#[event]
pub struct ConfigUpdated {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub base_fee_bps: u16,
    pub launch_fee_bps: u16,
    pub launch_window_slots: u64,
}

/// An admin transfer was proposed, or cancelled when `pending_admin` is the
/// default address.
#[event]
pub struct AdminProposed {
    pub admin: Pubkey,
    pub pending_admin: Pubkey,
}

#[event]
pub struct AdminChanged {
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
}
