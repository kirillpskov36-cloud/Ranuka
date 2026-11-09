use anchor_lang::prelude::*;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";

/// Layout version written into every account this program creates.
pub const ACCOUNT_VERSION: u8 = 1;

/// `BPFLoaderUpgradeab1e11111111111111111111111`, the loader that owns every
/// upgradeable program's ProgramData account.
pub const UPGRADEABLE_LOADER_ID: Pubkey = Pubkey::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 176, 226, 16, 21, 62, 247, 99, 174, 43, 0, 194, 185, 61, 22,
    193, 36, 210, 192, 83, 122, 16, 4, 128, 0, 0,
]);
