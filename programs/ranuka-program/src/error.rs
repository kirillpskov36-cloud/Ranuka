use anchor_lang::prelude::*;

/// Error codes are part of the public interface: append only, never reorder.
#[error_code]
pub enum RanukaError {
    #[msg("Signer is not authorized for this instruction")]
    Unauthorized,
    #[msg("Fee schedule is outside the hard caps")]
    InvalidFeeSchedule,
    #[msg("No admin transfer is pending")]
    NoPendingAdmin,
}
