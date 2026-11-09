//! Checked `a * b / d` in `u128`, with explicit rounding direction.

use crate::error::CoreError;

fn mul_div(a: u64, b: u64, denom: u128) -> Result<(u128, u128), CoreError> {
    let n = u128::from(a)
        .checked_mul(u128::from(b))
        .ok_or(CoreError::MathOverflow)?;
    let q = n.checked_div(denom).ok_or(CoreError::MathOverflow)?;
    let r = n.checked_rem(denom).ok_or(CoreError::MathOverflow)?;
    Ok((q, r))
}

/// `floor(a * b / denom)`. A zero denominator is reported as `MathOverflow`.
pub(crate) fn mul_div_floor(a: u64, b: u64, denom: u128) -> Result<u64, CoreError> {
    let (q, _) = mul_div(a, b, denom)?;
    u64::try_from(q).map_err(|_| CoreError::MathOverflow)
}

/// `ceil(a * b / denom)`. A zero denominator is reported as `MathOverflow`.
pub(crate) fn mul_div_ceil(a: u64, b: u64, denom: u128) -> Result<u64, CoreError> {
    let (q, r) = mul_div(a, b, denom)?;
    let q = if r == 0 {
        q
    } else {
        q.checked_add(1).ok_or(CoreError::MathOverflow)?
    };
    u64::try_from(q).map_err(|_| CoreError::MathOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_direction() {
        assert_eq!(mul_div_floor(7, 3, 2), Ok(10));
        assert_eq!(mul_div_ceil(7, 3, 2), Ok(11));
        assert_eq!(mul_div_floor(8, 3, 2), Ok(12));
        assert_eq!(mul_div_ceil(8, 3, 2), Ok(12));
    }

    #[test]
    fn zero_denominator_and_overflow_are_errors() {
        assert_eq!(mul_div_floor(1, 1, 0), Err(CoreError::MathOverflow));
        assert_eq!(mul_div_ceil(1, 1, 0), Err(CoreError::MathOverflow));
        assert_eq!(
            mul_div_floor(u64::MAX, u64::MAX, 1),
            Err(CoreError::MathOverflow)
        );
    }
}
