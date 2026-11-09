//! Ranuka on-chain program.
//!
//! Holds the money and the state, checks authority, moves funds and emits
//! events. It contains no pricing formulas of its own: all math comes from
//! `ranuka-core`. Lives under `programs/` because that is where Anchor looks.

#![deny(clippy::arithmetic_side_effects)]

pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

// Placeholder id; replaced by the real one at the first devnet deploy.
declare_id!("FG1nc4Q59Z78LvSYq97JMGhBCH2o4yzzvuM1K8xUt6NU");

#[program]
pub mod ranuka_program {
    use super::*;

    /// Creates the global config. Callable once, by the upgrade authority only.
    pub fn initialize(ctx: Context<Initialize>, fee_recipient: Pubkey) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, fee_recipient)
    }

    /// Updates the fee schedule and the fee recipient, within the hard caps.
    pub fn set_config(ctx: Context<SetConfig>, args: SetConfigArgs) -> Result<()> {
        crate::instructions::set_config::handle_set_config(ctx, args)
    }

    /// Step one of an admin transfer. The default address cancels it.
    pub fn propose_admin(ctx: Context<ProposeAdmin>, new_admin: Pubkey) -> Result<()> {
        crate::instructions::admin::handle_propose_admin(ctx, new_admin)
    }

    /// Step two of an admin transfer: the proposed admin takes over.
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        crate::instructions::admin::handle_accept_admin(ctx)
    }
}
