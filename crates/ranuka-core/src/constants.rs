//! Protocol constants. All amounts are integers in the smallest unit:
//! SOL in lamports, tokens in base units.

/// Lamports per SOL.
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Decimals of every Ranuka token.
pub const TOKEN_DECIMALS: u8 = 6;

/// Base units per whole token: `10^TOKEN_DECIMALS`.
pub const TOKEN_UNIT: u64 = 1_000_000;

/// Basis points in 100%.
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Default trade fee: 30 bps = 0.3%.
pub const DEFAULT_FEE_BPS: u16 = 30;

/// Hard cap on the base fee: 100 bps = 1%. Not even the admin can exceed it.
pub const MAX_FEE_BPS: u16 = 100;

/// Default buy fee in the creation slot: 5000 bps = 50%.
pub const DEFAULT_LAUNCH_FEE_BPS: u16 = 5_000;

/// Hard cap on the launch fee: 5000 bps = 50%.
pub const MAX_LAUNCH_FEE_BPS: u16 = 5_000;

/// Default length of the launch window: 150 slots, about one minute.
pub const DEFAULT_LAUNCH_WINDOW_SLOTS: u64 = 150;

/// Hard cap on the launch window: 1500 slots, about ten minutes.
/// Keeps the elevated fee from ever becoming permanent.
pub const MAX_LAUNCH_WINDOW_SLOTS: u64 = 1_500;
