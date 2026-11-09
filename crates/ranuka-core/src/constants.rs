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

/// Hard fee cap: 100 bps = 1%. Not even the admin can exceed it.
pub const MAX_FEE_BPS: u16 = 100;
