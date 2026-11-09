use anchor_lang::prelude::*;
use ranuka_core::{CurveParams, FeeSchedule};

use crate::{constants::*, error::RanukaError, events::ConfigUpdated, state::Global};

#[event_cpi]
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = Global::SPACE,
        seeds = [GLOBAL_SEED],
        bump
    )]
    pub global: Account<'info, Global>,
    /// CHECK: pinned by seeds to this program's ProgramData address; its owner
    /// and contents are verified in the handler.
    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        seeds::program = UPGRADEABLE_LOADER_ID
    )]
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Reads the upgrade authority out of a ProgramData account.
///
/// Layout (`UpgradeableLoaderState::ProgramData`): `u32` tag = 3, `u64` slot,
/// then `Option<Pubkey>`: one tag byte followed by the 32-byte key.
/// Returns `None` for anything that is not a ProgramData account with an
/// authority set, including immutable programs.
fn upgrade_authority(program_data: &AccountInfo) -> Option<Pubkey> {
    if program_data.owner != &UPGRADEABLE_LOADER_ID {
        return None;
    }
    let data = program_data.try_borrow_data().ok()?;
    if data.get(..4)? != [3, 0, 0, 0] || *data.get(12)? != 1 {
        return None;
    }
    let key: [u8; 32] = data.get(13..45)?.try_into().ok()?;
    Some(Pubkey::new_from_array(key))
}

/// Only the program's upgrade authority may initialize. Without this check,
/// anyone watching the deploy could call `initialize` first and become admin.
pub fn handle_initialize(ctx: Context<Initialize>, fee_recipient: Pubkey) -> Result<()> {
    let admin = ctx.accounts.admin.key();
    require!(
        upgrade_authority(&ctx.accounts.program_data) == Some(admin),
        RanukaError::Unauthorized
    );

    let schedule = FeeSchedule::DEFAULT;
    let params = CurveParams::DEFAULT;
    ctx.accounts.global.set_inner(Global {
        version: ACCOUNT_VERSION,
        bump: ctx.bumps.global,
        admin,
        pending_admin: Pubkey::default(),
        fee_recipient,
        base_fee_bps: schedule.base_fee_bps,
        launch_fee_bps: schedule.launch_fee_bps,
        launch_window_slots: schedule.launch_window_slots,
        initial_virtual_sol: params.initial_virtual_sol,
        initial_virtual_token: params.initial_virtual_token,
        initial_real_token: params.initial_real_token,
        total_supply: params.total_supply,
        reserved: [0; 64],
    });

    emit_cpi!(ConfigUpdated {
        admin,
        fee_recipient,
        base_fee_bps: schedule.base_fee_bps,
        launch_fee_bps: schedule.launch_fee_bps,
        launch_window_slots: schedule.launch_window_slots,
    });
    Ok(())
}
