//! Ranuka bonding-curve math.
//!
//! Pure functions only: numbers in, numbers out. No network, no Solana types.
//! The on-chain program and the SDK both call this crate, so a client-side
//! quote matches the on-chain result exactly.
//!
//! Rules (spec section 3):
//! - integers only: `u64` at the boundary, `u128` for intermediates;
//! - checked arithmetic only; overflow is an error, never a wrap or a panic;
//! - rounding always favors the pool.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
// Enforces the "checked arithmetic only" rule mechanically.
#![deny(clippy::arithmetic_side_effects)]
#![cfg_attr(test, allow(clippy::arithmetic_side_effects))]

pub mod constants;
mod curve;
mod error;
mod fee;
mod math;
mod types;

pub use curve::{fee_for, quote_buy_exact_in, quote_buy_exact_out, quote_sell};
pub use error::CoreError;
pub use fee::FeeSchedule;
pub use types::{CurveParams, Quote, Reserves};
