//! Two-step admin transfer: the current admin proposes, the new admin accepts.
//! A typo in the proposed address can be corrected; it can never lock the
//! program out of its own config.

use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::RanukaError,
    events::{AdminChanged, AdminProposed},
    state::Global,
};

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeAdmin<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = global.bump,
        has_one = admin @ RanukaError::Unauthorized
    )]
    pub global: Account<'info, Global>,
}

pub fn handle_propose_admin(ctx: Context<ProposeAdmin>, new_admin: Pubkey) -> Result<()> {
    let global = &mut ctx.accounts.global;
    global.pending_admin = new_admin;
    let admin = global.admin;

    emit_cpi!(AdminProposed {
        admin,
        pending_admin: new_admin,
    });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [GLOBAL_SEED], bump = global.bump)]
    pub global: Account<'info, Global>,
}

pub fn handle_accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
    let new_admin = ctx.accounts.new_admin.key();
    let global = &mut ctx.accounts.global;

    require!(
        global.pending_admin != Pubkey::default(),
        RanukaError::NoPendingAdmin
    );
    require_keys_eq!(global.pending_admin, new_admin, RanukaError::Unauthorized);

    let old_admin = global.admin;
    global.admin = new_admin;
    global.pending_admin = Pubkey::default();

    emit_cpi!(AdminChanged {
        old_admin,
        new_admin,
    });
    Ok(())
}
