//! Ranuka SDK: the bot-facing API.
//!
//! Placeholder. Planned modules (spec section 6):
//! `pda`, `ix`, `accounts`, `events` (offline) and `stream`, `client`
//! (behind the `rpc` feature).

/// Curve math, the same code the program runs; quotes need no RPC call.
pub use ranuka_core as math;
