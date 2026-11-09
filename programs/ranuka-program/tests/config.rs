//! Global config: initialization, fee schedule updates and admin transfer.
//! Runs the compiled program in LiteSVM; build it first with
//! `anchor build --ignore-keys` (scripts/check.sh does this when Anchor is installed).

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    ranuka_core::{CurveParams, FeeSchedule},
    ranuka_program::{
        constants::{GLOBAL_SEED, UPGRADEABLE_LOADER_ID},
        error::RanukaError,
        instructions::SetConfigArgs,
        state::Global,
    },
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const SOL: u64 = 1_000_000_000;

struct Env {
    svm: LiteSVM,
    program_id: Pubkey,
    /// Holds the program's upgrade authority.
    deployer: Keypair,
    global: Pubkey,
    program_data: Pubkey,
    event_authority: Pubkey,
}

/// Writes a ProgramData account with the given upgrade authority, the way the
/// upgradeable loader lays it out: tag 3, slot, then the optional authority.
fn set_program_data(svm: &mut LiteSVM, address: Pubkey, authority: Option<Pubkey>) {
    let mut data = vec![3u8, 0, 0, 0];
    data.extend_from_slice(&0u64.to_le_bytes());
    match authority {
        Some(key) => {
            data.push(1);
            data.extend_from_slice(key.as_ref());
        }
        None => {
            data.push(0);
            data.extend_from_slice(&[0u8; 32]);
        }
    }

    // Fund the address on first use so there is an account value to edit.
    if svm.get_account(&address).is_none() {
        svm.airdrop(&address, SOL).unwrap();
    }
    let mut account = svm.get_account(&address).unwrap();
    account.data = data;
    account.owner = UPGRADEABLE_LOADER_ID;
    svm.set_account(address, account).unwrap();
}

fn setup() -> Env {
    let program_id = ranuka_program::id();
    let mut svm = LiteSVM::new();
    // Loaded at run time, so the tests still compile before the program is built.
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/ranuka_program.so");
    let bytes = std::fs::read(path).unwrap_or_else(|_| {
        panic!("compiled program not found at {path}; run `anchor build --ignore-keys` first")
    });
    svm.add_program(program_id, &bytes).unwrap();

    let deployer = Keypair::new();
    svm.airdrop(&deployer.pubkey(), 10 * SOL).unwrap();

    let global = Pubkey::find_program_address(&[GLOBAL_SEED], &program_id).0;
    let program_data =
        Pubkey::find_program_address(&[program_id.as_ref()], &UPGRADEABLE_LOADER_ID).0;
    let event_authority = Pubkey::find_program_address(&[b"__event_authority"], &program_id).0;
    set_program_data(&mut svm, program_data, Some(deployer.pubkey()));

    Env {
        svm,
        program_id,
        deployer,
        global,
        program_data,
        event_authority,
    }
}

impl Env {
    fn funded_keypair(&mut self) -> Keypair {
        let keypair = Keypair::new();
        self.svm.airdrop(&keypair.pubkey(), 10 * SOL).unwrap();
        keypair
    }

    /// Sends one instruction signed and paid for by `signer`.
    /// On failure returns the error and the program logs as text.
    fn send(&mut self, instruction: Instruction, signer: &Keypair) -> Result<(), String> {
        self.svm.expire_blockhash();
        let blockhash = self.svm.latest_blockhash();
        let message =
            Message::new_with_blockhash(&[instruction], Some(&signer.pubkey()), &blockhash);
        let tx =
            VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[signer]).unwrap();
        self.svm
            .send_transaction(tx)
            .map(|_| ())
            .map_err(|failed| format!("{:?} | {}", failed.err, failed.meta.logs.join(" | ")))
    }

    fn global_state(&self) -> Global {
        let account = self.svm.get_account(&self.global).unwrap();
        let mut data: &[u8] = &account.data;
        Global::try_deserialize(&mut data).unwrap()
    }

    fn ix_initialize(
        &self,
        admin: Pubkey,
        program_data: Pubkey,
        fee_recipient: Pubkey,
    ) -> Instruction {
        Instruction::new_with_bytes(
            self.program_id,
            &ranuka_program::instruction::Initialize { fee_recipient }.data(),
            ranuka_program::accounts::Initialize {
                admin,
                global: self.global,
                program_data,
                system_program: system_program::ID,
                event_authority: self.event_authority,
                program: self.program_id,
            }
            .to_account_metas(None),
        )
    }

    fn ix_set_config(&self, admin: Pubkey, args: SetConfigArgs) -> Instruction {
        Instruction::new_with_bytes(
            self.program_id,
            &ranuka_program::instruction::SetConfig { args }.data(),
            ranuka_program::accounts::SetConfig {
                admin,
                global: self.global,
                event_authority: self.event_authority,
                program: self.program_id,
            }
            .to_account_metas(None),
        )
    }

    fn ix_propose_admin(&self, admin: Pubkey, new_admin: Pubkey) -> Instruction {
        Instruction::new_with_bytes(
            self.program_id,
            &ranuka_program::instruction::ProposeAdmin { new_admin }.data(),
            ranuka_program::accounts::ProposeAdmin {
                admin,
                global: self.global,
                event_authority: self.event_authority,
                program: self.program_id,
            }
            .to_account_metas(None),
        )
    }

    fn ix_accept_admin(&self, new_admin: Pubkey) -> Instruction {
        Instruction::new_with_bytes(
            self.program_id,
            &ranuka_program::instruction::AcceptAdmin {}.data(),
            ranuka_program::accounts::AcceptAdmin {
                new_admin,
                global: self.global,
                event_authority: self.event_authority,
                program: self.program_id,
            }
            .to_account_metas(None),
        )
    }

    /// Initializes with the deployer as admin and returns the fee recipient.
    fn initialize(&mut self) -> Pubkey {
        let fee_recipient = Keypair::new().pubkey();
        let deployer = self.deployer.insecure_clone();
        let ix = self.ix_initialize(deployer.pubkey(), self.program_data, fee_recipient);
        self.send(ix, &deployer).unwrap();
        fee_recipient
    }
}

/// Asserts that the transaction failed with exactly this custom error code.
#[track_caller]
fn assert_custom_error(result: Result<(), String>, code: u32) {
    let error = result.expect_err("transaction unexpectedly succeeded");
    assert!(
        error.contains(&format!("Custom({code})")),
        "expected custom error {code}, got: {error}"
    );
}

fn unauthorized() -> u32 {
    RanukaError::Unauthorized.into()
}

fn default_args(fee_recipient: Pubkey) -> SetConfigArgs {
    let schedule = FeeSchedule::DEFAULT;
    SetConfigArgs {
        base_fee_bps: schedule.base_fee_bps,
        launch_fee_bps: schedule.launch_fee_bps,
        launch_window_slots: schedule.launch_window_slots,
        fee_recipient,
    }
}

#[test]
fn initialize_writes_default_config() {
    let mut env = setup();
    let fee_recipient = env.initialize();

    let global = env.global_state();
    assert_eq!(global.version, 1);
    assert_eq!(global.admin, env.deployer.pubkey());
    assert_eq!(global.pending_admin, Pubkey::default());
    assert_eq!(global.fee_recipient, fee_recipient);
    assert_eq!(global.fee_schedule(), FeeSchedule::DEFAULT);

    let params = CurveParams::DEFAULT;
    assert_eq!(global.initial_virtual_sol, params.initial_virtual_sol);
    assert_eq!(global.initial_virtual_token, params.initial_virtual_token);
    assert_eq!(global.initial_real_token, params.initial_real_token);
    assert_eq!(global.total_supply, params.total_supply);

    // The account is exactly as large as its declared layout.
    let account = env.svm.get_account(&env.global).unwrap();
    assert_eq!(account.data.len(), Global::SPACE);
    assert_eq!(account.owner, env.program_id);
}

#[test]
fn initialize_rejects_anyone_but_the_upgrade_authority() {
    let mut env = setup();
    let stranger = env.funded_keypair();

    let ix = env.ix_initialize(stranger.pubkey(), env.program_data, stranger.pubkey());
    assert_custom_error(env.send(ix, &stranger), unauthorized());
}

#[test]
fn initialize_rejects_a_substituted_program_data_account() {
    let mut env = setup();
    let stranger = env.funded_keypair();

    // A look-alike account that names the stranger as upgrade authority.
    let fake = Keypair::new().pubkey();
    set_program_data(&mut env.svm, fake, Some(stranger.pubkey()));

    let ix = env.ix_initialize(stranger.pubkey(), fake, stranger.pubkey());
    let seeds_mismatch: u32 = anchor_lang::error::ErrorCode::ConstraintSeeds.into();
    assert_custom_error(env.send(ix, &stranger), seeds_mismatch);
}

#[test]
fn initialize_rejects_program_data_not_owned_by_the_loader() {
    let mut env = setup();

    // Right address and right bytes, but owned by the system program.
    let mut account = env.svm.get_account(&env.program_data).unwrap();
    account.owner = system_program::ID;
    env.svm.set_account(env.program_data, account).unwrap();

    let deployer = env.deployer.insecure_clone();
    let ix = env.ix_initialize(deployer.pubkey(), env.program_data, deployer.pubkey());
    assert_custom_error(env.send(ix, &deployer), unauthorized());
}

#[test]
fn immutable_program_cannot_be_initialized() {
    let mut env = setup();
    set_program_data(&mut env.svm, env.program_data, None);

    let deployer = env.deployer.insecure_clone();
    let ix = env.ix_initialize(deployer.pubkey(), env.program_data, deployer.pubkey());
    assert_custom_error(env.send(ix, &deployer), unauthorized());
}

#[test]
fn initialize_runs_only_once() {
    let mut env = setup();
    let first_recipient = env.initialize();

    let deployer = env.deployer.insecure_clone();
    let ix = env.ix_initialize(deployer.pubkey(), env.program_data, Keypair::new().pubkey());
    assert!(env.send(ix, &deployer).is_err());
    assert_eq!(env.global_state().fee_recipient, first_recipient);
}

#[test]
fn admin_updates_fee_schedule_and_recipient() {
    let mut env = setup();
    env.initialize();

    let args = SetConfigArgs {
        base_fee_bps: 50,
        launch_fee_bps: 500,
        launch_window_slots: 100,
        fee_recipient: Keypair::new().pubkey(),
    };
    let deployer = env.deployer.insecure_clone();
    let ix = env.ix_set_config(deployer.pubkey(), args);
    env.send(ix, &deployer).unwrap();

    let global = env.global_state();
    assert_eq!(global.base_fee_bps, 50);
    assert_eq!(global.launch_fee_bps, 500);
    assert_eq!(global.launch_window_slots, 100);
    assert_eq!(global.fee_recipient, args.fee_recipient);
}

#[test]
fn set_config_rejects_non_admin() {
    let mut env = setup();
    let fee_recipient = env.initialize();
    let stranger = env.funded_keypair();

    let ix = env.ix_set_config(stranger.pubkey(), default_args(stranger.pubkey()));
    assert_custom_error(env.send(ix, &stranger), unauthorized());
    assert_eq!(env.global_state().fee_recipient, fee_recipient);
}

#[test]
fn set_config_enforces_hard_caps() {
    let mut env = setup();
    let fee_recipient = env.initialize();
    let deployer = env.deployer.insecure_clone();
    let ok = default_args(fee_recipient);

    let out_of_bounds = [
        // Base fee above 1%.
        SetConfigArgs {
            base_fee_bps: 101,
            ..ok
        },
        // Launch fee above 10%.
        SetConfigArgs {
            launch_fee_bps: 1_001,
            ..ok
        },
        // Launch window above 150 slots.
        SetConfigArgs {
            launch_window_slots: 151,
            ..ok
        },
        // Launch fee below the base fee.
        SetConfigArgs {
            base_fee_bps: 100,
            launch_fee_bps: 50,
            ..ok
        },
    ];
    for args in out_of_bounds {
        let ix = env.ix_set_config(deployer.pubkey(), args);
        assert_custom_error(
            env.send(ix, &deployer),
            RanukaError::InvalidFeeSchedule.into(),
        );
    }
    assert_eq!(env.global_state().fee_schedule(), FeeSchedule::DEFAULT);

    // The caps themselves are allowed.
    let at_caps = SetConfigArgs {
        base_fee_bps: 100,
        launch_fee_bps: 1_000,
        launch_window_slots: 150,
        ..ok
    };
    let ix = env.ix_set_config(deployer.pubkey(), at_caps);
    env.send(ix, &deployer).unwrap();
}

#[test]
fn admin_transfer_takes_two_steps() {
    let mut env = setup();
    let fee_recipient = env.initialize();
    let deployer = env.deployer.insecure_clone();
    let successor = env.funded_keypair();
    let stranger = env.funded_keypair();

    // Nothing to accept before a proposal.
    let ix = env.ix_accept_admin(successor.pubkey());
    assert_custom_error(env.send(ix, &successor), RanukaError::NoPendingAdmin.into());

    // Only the admin can propose.
    let ix = env.ix_propose_admin(stranger.pubkey(), stranger.pubkey());
    assert_custom_error(env.send(ix, &stranger), unauthorized());

    let ix = env.ix_propose_admin(deployer.pubkey(), successor.pubkey());
    env.send(ix, &deployer).unwrap();
    assert_eq!(env.global_state().pending_admin, successor.pubkey());
    // Proposing alone changes nothing: the old admin is still in charge.
    assert_eq!(env.global_state().admin, deployer.pubkey());

    // Only the proposed admin can accept.
    let ix = env.ix_accept_admin(stranger.pubkey());
    assert_custom_error(env.send(ix, &stranger), unauthorized());

    let ix = env.ix_accept_admin(successor.pubkey());
    env.send(ix, &successor).unwrap();
    let global = env.global_state();
    assert_eq!(global.admin, successor.pubkey());
    assert_eq!(global.pending_admin, Pubkey::default());

    // The old admin has lost control; the new one has it.
    let args = default_args(fee_recipient);
    let ix = env.ix_set_config(deployer.pubkey(), args);
    assert_custom_error(env.send(ix, &deployer), unauthorized());
    let ix = env.ix_set_config(successor.pubkey(), args);
    env.send(ix, &successor).unwrap();
}

#[test]
fn admin_can_cancel_a_pending_transfer() {
    let mut env = setup();
    env.initialize();
    let deployer = env.deployer.insecure_clone();
    let successor = env.funded_keypair();

    let ix = env.ix_propose_admin(deployer.pubkey(), successor.pubkey());
    env.send(ix, &deployer).unwrap();
    let ix = env.ix_propose_admin(deployer.pubkey(), Pubkey::default());
    env.send(ix, &deployer).unwrap();

    let ix = env.ix_accept_admin(successor.pubkey());
    assert_custom_error(env.send(ix, &successor), RanukaError::NoPendingAdmin.into());
    assert_eq!(env.global_state().admin, deployer.pubkey());
}
