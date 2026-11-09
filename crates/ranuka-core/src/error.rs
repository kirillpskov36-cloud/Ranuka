//! Math errors. Functions in this crate never panic; every failure is a `CoreError`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    /// Input or output amount is zero.
    ZeroAmount,
    /// A result does not fit its integer type.
    MathOverflow,
    /// More tokens requested than remain on the curve.
    InsufficientLiquidity,
    /// Fee above `MAX_FEE_BPS`.
    FeeTooHigh,
    /// The curve is complete; trading on it is closed.
    CurveComplete,
}
