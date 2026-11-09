use anchor_lang::prelude::*;
use ranuka_core::FeeSchedule;

use crate::{constants::*, error::RanukaError, events::ConfigUpdated, state::Global};

/// The complete new config. Every call states all fields, so a change can
/// never silently keep a stale value.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetConfigArgs {
    pub base_fee_bps: u16,
    pub launch_fee_bps: u16,
    pub launch_window_slots: u64,
    pub fee_recipient: Pubkey,
}

#[event_cpi]
#[derive(Accounts)]
pub struct SetConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = global.bump,
        has_one = admin @ RanukaError::Unauthorized
    )]
    pub global: Account<'info, Global>,
}

/// The hard caps live in `ranuka-core`; the admin cannot move the schedule
/// outside them. Existing curves pick up the new schedule on their next trade.
pub fn handle_set_config(ctx: Context<SetConfig>, args: SetConfigArgs) -> Result<()> {
    let schedule = FeeSchedule {
        base_fee_bps: args.base_fee_bps,
        launch_fee_bps: args.launch_fee_bps,
        launch_window_slots: args.launch_window_slots,
    };
    require!(schedule.validate().is_ok(), RanukaError::InvalidFeeSchedule);

    let global = &mut ctx.accounts.global;
    global.set_fee_schedule(&schedule);
    global.fee_recipient = args.fee_recipient;
    let admin = global.admin;

    emit_cpi!(ConfigUpdated {
        admin,
        fee_recipient: args.fee_recipient,
        base_fee_bps: args.base_fee_bps,
        launch_fee_bps: args.launch_fee_bps,
        launch_window_slots: args.launch_window_slots,
    });
    Ok(())
}
