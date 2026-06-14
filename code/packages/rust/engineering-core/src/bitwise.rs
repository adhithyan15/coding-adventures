//! # Bitwise operations on 48-bit unsigned integers.
//!
//! Excel's bitwise functions limit inputs to 48-bit unsigned integers
//! (`2^48 - 1` max, no negative values). Shift counts are in `0..=53`
//! (Excel's documented range).
//!
//! Returns are f64 in Excel; we follow.

use super::EngineeringError;

const MAX_48BIT: u64 = (1_u64 << 48) - 1;
const MAX_SHIFT: i64 = 53;

fn validate_u48(value: f64, function: &'static str) -> Result<u64, EngineeringError> {
    if value < 0.0 || value > MAX_48BIT as f64 {
        return Err(EngineeringError::OutOfRange {
            function,
            what: format!("value must be in [0, 2^48-1] ({value})"),
        });
    }
    if value.fract() != 0.0 {
        return Err(EngineeringError::DomainError {
            function,
            what: format!("value must be an integer ({value})"),
        });
    }
    Ok(value as u64)
}

/// Excel `BITAND(number1, number2)`.
pub fn bitand(a: f64, b: f64) -> Result<f64, EngineeringError> {
    let a = validate_u48(a, "bitand")?;
    let b = validate_u48(b, "bitand")?;
    Ok((a & b) as f64)
}

/// Excel `BITOR(number1, number2)`.
pub fn bitor(a: f64, b: f64) -> Result<f64, EngineeringError> {
    let a = validate_u48(a, "bitor")?;
    let b = validate_u48(b, "bitor")?;
    Ok((a | b) as f64)
}

/// Excel `BITXOR(number1, number2)`.
pub fn bitxor(a: f64, b: f64) -> Result<f64, EngineeringError> {
    let a = validate_u48(a, "bitxor")?;
    let b = validate_u48(b, "bitxor")?;
    Ok((a ^ b) as f64)
}

/// Excel `BITLSHIFT(number, shift_amount)`. Negative shift becomes a
/// right shift. Result must fit in 48 bits.
pub fn bitlshift(value: f64, shift: f64) -> Result<f64, EngineeringError> {
    let v = validate_u48(value, "bitlshift")?;
    if shift.fract() != 0.0 || shift.abs() > MAX_SHIFT as f64 {
        return Err(EngineeringError::BadParameter {
            name: "shift",
            value: shift.to_string(),
        });
    }
    let s = shift as i64;
    let result = if s >= 0 {
        let shifted = v.checked_shl(s as u32).unwrap_or(0);
        shifted & MAX_48BIT
    } else {
        v.checked_shr((-s) as u32).unwrap_or(0)
    };
    if result > MAX_48BIT {
        return Err(EngineeringError::OutOfRange {
            function: "bitlshift",
            what: format!("result exceeds 2^48-1 ({result})"),
        });
    }
    Ok(result as f64)
}

/// Excel `BITRSHIFT(number, shift_amount)`. Negative shift becomes a
/// left shift.
pub fn bitrshift(value: f64, shift: f64) -> Result<f64, EngineeringError> {
    // BITRSHIFT(x, n) = BITLSHIFT(x, -n).
    bitlshift(value, -shift)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitand_simple() {
        // 0b1100 & 0b1010 = 0b1000 = 8
        assert_eq!(bitand(12.0, 10.0).unwrap(), 8.0);
    }

    #[test]
    fn bitor_simple() {
        // 0b1100 | 0b1010 = 0b1110 = 14
        assert_eq!(bitor(12.0, 10.0).unwrap(), 14.0);
    }

    #[test]
    fn bitxor_simple() {
        // 0b1100 ^ 0b1010 = 0b0110 = 6
        assert_eq!(bitxor(12.0, 10.0).unwrap(), 6.0);
    }

    #[test]
    fn bitlshift_basic() {
        assert_eq!(bitlshift(1.0, 8.0).unwrap(), 256.0);
        assert_eq!(bitlshift(256.0, -1.0).unwrap(), 128.0); // negative = right
    }

    #[test]
    fn bitrshift_basic() {
        assert_eq!(bitrshift(256.0, 1.0).unwrap(), 128.0);
        assert_eq!(bitrshift(1.0, -8.0).unwrap(), 256.0); // negative = left
    }

    #[test]
    fn negative_value_rejected() {
        assert!(bitand(-1.0, 1.0).is_err());
        assert!(bitor(-1.0, 1.0).is_err());
    }

    #[test]
    fn out_of_48bit_rejected() {
        let too_big = (1_u64 << 48) as f64;
        assert!(bitand(too_big, 1.0).is_err());
    }

    #[test]
    fn fractional_value_rejected() {
        assert!(bitand(1.5, 1.0).is_err());
    }

    #[test]
    fn shift_too_large_rejected() {
        assert!(bitlshift(1.0, 54.0).is_err());
        assert!(bitlshift(1.0, -54.0).is_err());
    }
}
