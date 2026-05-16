//! # Base conversions — BIN/OCT/DEC/HEX.
//!
//! Excel's base-conversion functions accept input as either a number
//! or a string, and always return a string (so leading zeros from
//! `places` padding survive). Each base has a documented bit-width
//! limit and a two's-complement representation for negative values.
//!
//! Bit-widths and ranges (Excel-documented):
//! - BIN: 10 bits, range `-2^9 .. 2^9 - 1` = `-512 .. 511`
//! - OCT: 30 bits, range `-2^29 .. 2^29 - 1`
//! - HEX: 40 bits, range `-2^39 .. 2^39 - 1`
//!
//! The negative encoding is two's complement padded to the bit-width
//! of the *target* base: e.g. `DEC2BIN(-1)` returns `"1111111111"`,
//! `DEC2HEX(-1)` returns `"FFFFFFFFFF"`.

use super::EngineeringError;

const BIN_BITS: u32 = 10;
const OCT_BITS: u32 = 30;
const HEX_BITS: u32 = 40;

const BIN_MASK: i64 = (1_i64 << BIN_BITS) - 1; // 0x3FF
const OCT_MASK: i64 = (1_i64 << OCT_BITS) - 1;
const HEX_MASK: i64 = (1_i64 << HEX_BITS) - 1;

const BIN_MIN: i64 = -(1_i64 << (BIN_BITS - 1));
const BIN_MAX: i64 = (1_i64 << (BIN_BITS - 1)) - 1;
const OCT_MIN: i64 = -(1_i64 << (OCT_BITS - 1));
const OCT_MAX: i64 = (1_i64 << (OCT_BITS - 1)) - 1;
const HEX_MIN: i64 = -(1_i64 << (HEX_BITS - 1));
const HEX_MAX: i64 = (1_i64 << (HEX_BITS - 1)) - 1;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a sign-extended value from a base-`radix` string under
/// Excel's constraints. The input string represents a number in the
/// stated base; negative values are encoded as two's complement
/// padded to `bits` bits.
fn parse_signed(
    input: &str,
    radix: u32,
    bits: u32,
    mask: i64,
    function: &'static str,
) -> Result<i64, EngineeringError> {
    if input.is_empty() {
        return Err(EngineeringError::ParseError {
            function,
            input: input.to_string(),
        });
    }
    if input.len() > 10 {
        return Err(EngineeringError::OutOfRange {
            function,
            what: format!("input exceeds 10 characters ({})", input.len()),
        });
    }
    let raw = i64::from_str_radix(input, radix).map_err(|_| EngineeringError::ParseError {
        function,
        input: input.to_string(),
    })?;
    if raw < 0 {
        // Excel doesn't accept a literal "-" prefix in BIN2/OCT2/HEX2.
        return Err(EngineeringError::ParseError {
            function,
            input: input.to_string(),
        });
    }
    // Check the high bit (bit `bits - 1`) of raw; if set, treat as
    // negative under two's complement.
    let sign_bit = 1_i64 << (bits - 1);
    let value = if raw & sign_bit != 0 {
        raw - (mask + 1)
    } else {
        raw
    };
    Ok(value)
}

/// Format a signed value into base-`radix` text with the negative
/// two's-complement encoding and optional `places` zero-padding.
fn format_signed(
    value: i64,
    radix: u32,
    bits: u32,
    mask: i64,
    places: Option<u32>,
    function: &'static str,
) -> Result<String, EngineeringError> {
    let two_compl = if value < 0 {
        (value + (mask + 1)) & mask
    } else {
        value
    };
    // Build the base representation.
    let base_str = match radix {
        2 => format!("{:b}", two_compl),
        8 => format!("{:o}", two_compl),
        10 => format!("{}", two_compl),
        16 => format!("{:X}", two_compl),
        _ => unreachable!(),
    };
    // Negative numbers ALWAYS get exactly `bits / log2(radix)` chars
    // (Excel pads with leading 1s in BIN; leading F's in HEX).
    if value < 0 {
        let target_chars = match radix {
            2 => bits as usize,
            8 => (bits as f64 / 3.0).ceil() as usize,
            16 => (bits as f64 / 4.0).ceil() as usize,
            _ => base_str.len(),
        };
        return Ok(format!("{:0>width$}", base_str, width = target_chars));
    }
    // Positive: apply `places` padding when given. Excel returns
    // `#NUM!` if places is fewer than required digits.
    if let Some(p) = places {
        if (p as usize) < base_str.len() {
            return Err(EngineeringError::OutOfRange {
                function,
                what: format!("places ({p}) < required digits ({})", base_str.len()),
            });
        }
        return Ok(format!("{:0>width$}", base_str, width = p as usize));
    }
    Ok(base_str)
}

fn range_check(
    value: i64,
    min: i64,
    max: i64,
    function: &'static str,
) -> Result<(), EngineeringError> {
    if !(min..=max).contains(&value) {
        return Err(EngineeringError::OutOfRange {
            function,
            what: format!("value {value} not in [{min}, {max}]"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// DEC2*
// ---------------------------------------------------------------------------

/// Excel `DEC2BIN(number, [places])`.
pub fn dec2bin(number: i64, places: Option<u32>) -> Result<String, EngineeringError> {
    range_check(number, BIN_MIN, BIN_MAX, "dec2bin")?;
    format_signed(number, 2, BIN_BITS, BIN_MASK, places, "dec2bin")
}

/// Excel `DEC2OCT(number, [places])`.
pub fn dec2oct(number: i64, places: Option<u32>) -> Result<String, EngineeringError> {
    range_check(number, OCT_MIN, OCT_MAX, "dec2oct")?;
    format_signed(number, 8, OCT_BITS, OCT_MASK, places, "dec2oct")
}

/// Excel `DEC2HEX(number, [places])`.
pub fn dec2hex(number: i64, places: Option<u32>) -> Result<String, EngineeringError> {
    range_check(number, HEX_MIN, HEX_MAX, "dec2hex")?;
    format_signed(number, 16, HEX_BITS, HEX_MASK, places, "dec2hex")
}

// ---------------------------------------------------------------------------
// BIN2* / OCT2* / HEX2*
// ---------------------------------------------------------------------------

/// Excel `BIN2DEC(number)`.
pub fn bin2dec(input: &str) -> Result<i64, EngineeringError> {
    parse_signed(input, 2, BIN_BITS, BIN_MASK, "bin2dec")
}

/// Excel `BIN2OCT(number, [places])`.
pub fn bin2oct(input: &str, places: Option<u32>) -> Result<String, EngineeringError> {
    let v = bin2dec(input)?;
    format_signed(v, 8, OCT_BITS, OCT_MASK, places, "bin2oct")
}

/// Excel `BIN2HEX(number, [places])`.
pub fn bin2hex(input: &str, places: Option<u32>) -> Result<String, EngineeringError> {
    let v = bin2dec(input)?;
    format_signed(v, 16, HEX_BITS, HEX_MASK, places, "bin2hex")
}

/// Excel `OCT2DEC(number)`.
pub fn oct2dec(input: &str) -> Result<i64, EngineeringError> {
    parse_signed(input, 8, OCT_BITS, OCT_MASK, "oct2dec")
}

/// Excel `OCT2BIN(number, [places])`.
pub fn oct2bin(input: &str, places: Option<u32>) -> Result<String, EngineeringError> {
    let v = oct2dec(input)?;
    range_check(v, BIN_MIN, BIN_MAX, "oct2bin")?;
    format_signed(v, 2, BIN_BITS, BIN_MASK, places, "oct2bin")
}

/// Excel `OCT2HEX(number, [places])`.
pub fn oct2hex(input: &str, places: Option<u32>) -> Result<String, EngineeringError> {
    let v = oct2dec(input)?;
    format_signed(v, 16, HEX_BITS, HEX_MASK, places, "oct2hex")
}

/// Excel `HEX2DEC(number)`.
pub fn hex2dec(input: &str) -> Result<i64, EngineeringError> {
    parse_signed(input, 16, HEX_BITS, HEX_MASK, "hex2dec")
}

/// Excel `HEX2BIN(number, [places])`.
pub fn hex2bin(input: &str, places: Option<u32>) -> Result<String, EngineeringError> {
    let v = hex2dec(input)?;
    range_check(v, BIN_MIN, BIN_MAX, "hex2bin")?;
    format_signed(v, 2, BIN_BITS, BIN_MASK, places, "hex2bin")
}

/// Excel `HEX2OCT(number, [places])`.
pub fn hex2oct(input: &str, places: Option<u32>) -> Result<String, EngineeringError> {
    let v = hex2dec(input)?;
    range_check(v, OCT_MIN, OCT_MAX, "hex2oct")?;
    format_signed(v, 8, OCT_BITS, OCT_MASK, places, "hex2oct")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec2bin_positive_and_padded() {
        assert_eq!(dec2bin(9, None).unwrap(), "1001");
        assert_eq!(dec2bin(9, Some(4)).unwrap(), "1001");
        assert_eq!(dec2bin(9, Some(6)).unwrap(), "001001");
        // Excel rejects places < required digits.
        assert!(matches!(
            dec2bin(9, Some(2)),
            Err(EngineeringError::OutOfRange { .. })
        ));
    }

    #[test]
    fn dec2bin_negative_twos_complement_pads_to_10() {
        // -1 in 10 bits = 1111111111.
        assert_eq!(dec2bin(-1, None).unwrap(), "1111111111");
        // -2 in 10 bits = 1111111110.
        assert_eq!(dec2bin(-2, None).unwrap(), "1111111110");
        // -512 = 1000000000 (min).
        assert_eq!(dec2bin(-512, None).unwrap(), "1000000000");
    }

    #[test]
    fn dec2bin_out_of_range() {
        assert!(dec2bin(512, None).is_err());
        assert!(dec2bin(-513, None).is_err());
    }

    #[test]
    fn dec2hex_negative_twos_complement_pads_to_10_chars() {
        // -1 in 40 bits hex = FFFFFFFFFF (10 chars).
        assert_eq!(dec2hex(-1, None).unwrap(), "FFFFFFFFFF");
    }

    #[test]
    fn bin2dec_round_trips() {
        for v in [-512, -1, 0, 1, 255, 511] {
            let s = dec2bin(v, None).unwrap();
            let back = bin2dec(&s).unwrap();
            assert_eq!(back, v, "value {v}");
        }
    }

    #[test]
    fn hex2dec_round_trips() {
        for v in [-100_000_i64, -1, 0, 1, 1_000_000, 5_000_000_000] {
            let s = dec2hex(v, None).unwrap();
            let back = hex2dec(&s).unwrap();
            assert_eq!(back, v, "value {v}");
        }
    }

    #[test]
    fn oct2hex_chain() {
        // 100 in decimal -> 144 octal -> 64 hex
        assert_eq!(dec2oct(100, None).unwrap(), "144");
        assert_eq!(oct2hex("144", None).unwrap(), "64");
        assert_eq!(hex2dec("64").unwrap(), 100);
    }

    #[test]
    fn empty_string_rejected() {
        assert!(bin2dec("").is_err());
        assert!(hex2dec("").is_err());
    }

    #[test]
    fn negative_literal_rejected() {
        // Excel: BIN2DEC does not accept "-1" as input.
        assert!(bin2dec("-1").is_err());
        assert!(hex2dec("-FF").is_err());
    }

    #[test]
    fn too_long_input_rejected() {
        assert!(bin2dec("11111111111").is_err()); // 11 chars
        assert!(hex2dec("12345678901").is_err()); // 11 chars
    }

    #[test]
    fn bin2hex_handles_negative_chain() {
        // -1 binary (in 10 bits) = "1111111111"
        // -1 hex (in 10 chars) = "FFFFFFFFFF"
        assert_eq!(bin2hex("1111111111", None).unwrap(), "FFFFFFFFFF");
    }
}
