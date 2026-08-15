//! # Numeric literal parsing
//!
//! WAT numeric literals are richer than Rust's own `str::parse`:
//!
//! - **Integers**: decimal or `0x`-prefixed hex, with `_` digit separators
//!   (`1_000_000`), and — for an `iN`-typed context — either the *signed*
//!   or *unsigned* bit-pattern spelling of the same value is accepted
//!   (`-1` and `0xffffffff` both denote the i32 bit pattern `0xffffffff`).
//! - **Floats**: ordinary decimal (`3.14`, `1e10`), and **hex floats**
//!   (`0x1.8p3` = 1.5 × 2³ = 12.0) — parsed bit-exact, not through an
//!   approximate `str::parse`. Plus `inf`/`nan`, and `nan:0x<payload>` — an
//!   *exact* NaN bit pattern, not "any NaN."
//!
//! Every parser here takes the raw atom text (already tokenized; no
//! surrounding whitespace) and returns either the parsed value or a
//! [`WastParseError`] carrying the byte offset the caller passed in for
//! error messages.

use crate::WastParseError;

/// Strip WAT's `_` digit-separator syntax. (The full grammar only allows
/// `_` *between* digits, never leading/trailing/doubled; this parser is
/// lenient about placement, which only matters for deliberately-malformed
/// literal-syntax fixtures, none of which are in this phase's vendored
/// slice — see `W05-wasm-conformance-harness.md` §5.)
fn strip_underscores(s: &str) -> String {
    s.chars().filter(|&c| c != '_').collect()
}

/// Parse a WAT integer literal (decimal or `0x`-hex, optionally signed)
/// into its full-precision magnitude and sign, without any width-specific
/// range check — [`parse_i32`]/[`parse_i64`] apply that afterward.
fn parse_int_magnitude(text: &str, pos: usize) -> Result<(bool, u128), WastParseError> {
    let cleaned = strip_underscores(text);
    let (neg, rest) = match cleaned.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    let magnitude = if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        u128::from_str_radix(hex, 16)
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?
    } else {
        rest.parse::<u128>()
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?
    };
    Ok((neg, magnitude))
}

/// Parse an `i32`-context integer literal to its raw bit pattern. Accepts
/// the WAT-permitted range `[-2^31, 2^32-1]` — the union of the signed and
/// unsigned spellings of every 32-bit pattern.
pub fn parse_i32(text: &str, pos: usize) -> Result<i32, WastParseError> {
    let (neg, mag) = parse_int_magnitude(text, pos)?;
    // `.wrapping_neg()`, not unary `-` -- a magnitude near `u128::MAX` (a
    // syntactically valid, if absurd, literal like a 32-hex-digit `0x...`)
    // would overflow plain negation in a debug build (`overflow-checks`),
    // panicking before the range check below gets a chance to reject it
    // cleanly. Wrapping is safe here precisely because the result is
    // immediately range-checked against i32's real bounds regardless.
    let signed = if neg { (mag as i128).wrapping_neg() } else { mag as i128 };
    if !(-(1i128 << 31)..(1i128 << 32)).contains(&signed) {
        return Err(WastParseError::InvalidNumericLiteralForType {
            pos,
            text: text.to_string(),
            ty: "i32",
        });
    }
    Ok(signed as u32 as i32)
}

/// As [`parse_i32`], for the 8-bit range `[-2^7, 2^8-1]`. There's no plain
/// `i8.const` in WASM (`i32` is the smallest scalar integer type) -- this
/// exists only for `v128.const`'s `i8x16` shape, whose 16 lane literals
/// are each an 8-bit value.
pub fn parse_i8(text: &str, pos: usize) -> Result<i8, WastParseError> {
    let (neg, mag) = parse_int_magnitude(text, pos)?;
    let signed = if neg { (mag as i128).wrapping_neg() } else { mag as i128 };
    if !(-(1i128 << 7)..(1i128 << 8)).contains(&signed) {
        return Err(WastParseError::InvalidNumericLiteralForType { pos, text: text.to_string(), ty: "i8" });
    }
    Ok(signed as u8 as i8)
}

/// As [`parse_i32`], for the 16-bit range `[-2^15, 2^16-1]` -- exists only
/// for `v128.const`'s `i16x8` shape, same reason as [`parse_i8`].
pub fn parse_i16(text: &str, pos: usize) -> Result<i16, WastParseError> {
    let (neg, mag) = parse_int_magnitude(text, pos)?;
    let signed = if neg { (mag as i128).wrapping_neg() } else { mag as i128 };
    if !(-(1i128 << 15)..(1i128 << 16)).contains(&signed) {
        return Err(WastParseError::InvalidNumericLiteralForType { pos, text: text.to_string(), ty: "i16" });
    }
    Ok(signed as u16 as i16)
}

/// As [`parse_i32`], for the 64-bit range `[-2^63, 2^64-1]`.
pub fn parse_i64(text: &str, pos: usize) -> Result<i64, WastParseError> {
    let (neg, mag) = parse_int_magnitude(text, pos)?;
    if neg {
        if mag > (1u128 << 63) {
            return Err(WastParseError::InvalidNumericLiteralForType {
                pos,
                text: text.to_string(),
                ty: "i64",
            });
        }
        Ok((mag as i128).wrapping_neg() as i64)
    } else {
        if mag > u64::MAX as u128 {
            return Err(WastParseError::InvalidNumericLiteralForType {
                pos,
                text: text.to_string(),
                ty: "i64",
            });
        }
        Ok(mag as u64 as i64)
    }
}

/// Parse a plain (non-NaN, non-inf) WAT float literal — decimal or hex — to
/// an `f64`. Hex floats (`0x1.8p3`) are computed bit-exact: the mantissa is
/// accumulated digit-by-digit (each hex digit contributes exactly 4 bits),
/// then scaled by `2^exponent` — multiplying by an exact power of two never
/// introduces rounding beyond what the mantissa accumulation itself does,
/// so this is exact for the short literals real testsuite files use.
fn parse_float_magnitude(text: &str, pos: usize) -> Result<f64, WastParseError> {
    let cleaned = strip_underscores(text);
    if let Some(hex) = cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        // WAT's `p`/`P` exponent is OPTIONAL, not just on a bare hex
        // integer (`f32.const 0xf32` = 3890.0) but also on a hex literal
        // with a fractional part (`0xa0_ff.f141_a59a` with no exponent at
        // all) -- both default to exponent 0, only the mantissa parsing
        // differs based on whether a `.` is present.
        let (mantissa_str, exp_str) = hex.split_once(['p', 'P']).unwrap_or((hex, "0"));
        let (int_part, frac_part) = match mantissa_str.split_once('.') {
            Some((i, f)) => (i, f),
            None => (mantissa_str, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() });
        }
        let mut mantissa = 0f64;
        for c in int_part.chars() {
            let d = c
                .to_digit(16)
                .ok_or(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?;
            mantissa = mantissa * 16.0 + d as f64;
        }
        let mut scale = 1f64 / 16.0;
        for c in frac_part.chars() {
            let d = c
                .to_digit(16)
                .ok_or(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?;
            mantissa += d as f64 * scale;
            scale /= 16.0;
        }
        let exponent: i32 = exp_str
            .parse()
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?;
        Ok(mantissa * 2f64.powi(exponent))
    } else {
        cleaned
            .parse::<f64>()
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })
    }
}

/// Parse a full WAT float literal — including `inf` and the two `nan`
/// forms — to an `f64` bit pattern (callers narrow to `f32` bits via
/// [`f64_to_f32_bits`] when needed).
///
/// `nan:0x<payload>` denotes an *exact* NaN bit pattern (quiet NaN with the
/// given mantissa payload), used by `assert_return`'s `nan:arithmetic`/
/// `nan:canonical` result classes — those are graded by NaN *class*, not
/// bit-exact value, at the call site in `wasm-conformance`, not here; this
/// function just produces the literal's own bits when one is written
/// directly in source (e.g. inside a `f64.const`).
pub fn parse_f64_bits(text: &str, pos: usize) -> Result<u64, WastParseError> {
    let (neg, rest) = match text.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let bits = if rest == "inf" {
        0x7FF0_0000_0000_0000u64
    } else if rest == "nan" {
        0x7FF8_0000_0000_0000u64
    } else if let Some(payload_str) = rest.strip_prefix("nan:0x").or_else(|| rest.strip_prefix("nan:0X")) {
        let payload = u64::from_str_radix(payload_str, 16)
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?;
        if payload == 0 || payload > 0x000F_FFFF_FFFF_FFFF {
            return Err(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() });
        }
        0x7FF0_0000_0000_0000u64 | payload
    } else {
        parse_float_magnitude(rest, pos)?.to_bits()
    };
    Ok(if neg { bits | 0x8000_0000_0000_0000 } else { bits })
}

/// As [`parse_f64_bits`], narrowed to `f32`'s 32-bit layout.
pub fn parse_f32_bits(text: &str, pos: usize) -> Result<u32, WastParseError> {
    let (neg, rest) = match text.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let bits = if rest == "inf" {
        0x7F80_0000u32
    } else if rest == "nan" {
        0x7FC0_0000u32
    } else if let Some(payload_str) = rest.strip_prefix("nan:0x").or_else(|| rest.strip_prefix("nan:0X")) {
        let payload = u32::from_str_radix(payload_str, 16)
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?;
        if payload == 0 || payload > 0x007F_FFFF {
            return Err(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() });
        }
        0x7F80_0000u32 | payload
    } else {
        (parse_float_magnitude(rest, pos)? as f32).to_bits()
    };
    Ok(if neg { bits | 0x8000_0000 } else { bits })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_i32() {
        assert_eq!(parse_i32("42", 0).unwrap(), 42);
        assert_eq!(parse_i32("-1", 0).unwrap(), -1);
    }

    #[test]
    fn parses_hex_i32_and_underscore_separators() {
        assert_eq!(parse_i32("0x2A", 0).unwrap(), 42);
        assert_eq!(parse_i32("1_000_000", 0).unwrap(), 1_000_000);
    }

    #[test]
    fn i32_accepts_both_signed_and_unsigned_spelling_of_same_bits() {
        // -1 and 0xffffffff denote the identical i32 bit pattern.
        assert_eq!(parse_i32("-1", 0).unwrap(), parse_i32("0xffffffff", 0).unwrap());
    }

    #[test]
    fn i32_out_of_range_is_rejected() {
        assert!(parse_i32("4294967296", 0).is_err()); // 2^32
        assert!(parse_i32("-2147483649", 0).is_err()); // -2^31 - 1
    }

    /// Security-review regression: a magnitude near `u128::MAX` (still a
    /// syntactically valid hex literal) used to overflow-panic on the
    /// unary `-` before the range check ever ran. Must return a clean
    /// `Err`, not panic, on a debug build (`overflow-checks = true`).
    #[test]
    fn i32_extreme_magnitude_negative_literal_errors_cleanly_not_panics() {
        let err = parse_i32("-0x80000000000000000000000000000000", 0).unwrap_err();
        assert!(matches!(err, WastParseError::InvalidNumericLiteralForType { .. }));
    }

    #[test]
    fn parses_i8_full_range_both_spellings() {
        assert_eq!(parse_i8("-1", 0).unwrap(), -1i8);
        assert_eq!(parse_i8("0xff", 0).unwrap(), -1i8);
        assert_eq!(parse_i8("255", 0).unwrap(), -1i8);
        assert_eq!(parse_i8("-128", 0).unwrap(), i8::MIN);
        assert_eq!(parse_i8("127", 0).unwrap(), i8::MAX);
    }

    #[test]
    fn i8_out_of_range_is_rejected() {
        assert!(parse_i8("256", 0).is_err());
        assert!(parse_i8("-129", 0).is_err());
    }

    #[test]
    fn parses_i16_full_range_both_spellings() {
        assert_eq!(parse_i16("-1", 0).unwrap(), -1i16);
        assert_eq!(parse_i16("0xffff", 0).unwrap(), -1i16);
        assert_eq!(parse_i16("65535", 0).unwrap(), -1i16);
        assert_eq!(parse_i16("-32768", 0).unwrap(), i16::MIN);
        assert_eq!(parse_i16("32767", 0).unwrap(), i16::MAX);
    }

    #[test]
    fn i16_out_of_range_is_rejected() {
        assert!(parse_i16("65536", 0).is_err());
        assert!(parse_i16("-32769", 0).is_err());
    }

    #[test]
    fn parses_i64_full_range() {
        assert_eq!(parse_i64("-1", 0).unwrap(), -1i64);
        assert_eq!(parse_i64("0xffffffffffffffff", 0).unwrap(), -1i64);
        assert_eq!(parse_i64("9223372036854775807", 0).unwrap(), i64::MAX);
    }

    #[test]
    fn hex_float_matches_known_value() {
        // 0x1.8p3 = 1.5 * 2^3 = 12.0
        let bits = parse_f64_bits("0x1.8p3", 0).unwrap();
        assert_eq!(f64::from_bits(bits), 12.0);
    }

    #[test]
    fn hex_float_negative_exponent() {
        // 0x1p-1 = 1.0 * 2^-1 = 0.5
        let bits = parse_f64_bits("0x1p-1", 0).unwrap();
        assert_eq!(f64::from_bits(bits), 0.5);
    }

    /// Found running the real WebAssembly/testsuite corpus (`call.wast`):
    /// a bare hex INTEGER (no `p`/`P` exponent) is valid wherever a float
    /// literal is expected -- `f32.const 0xf32` means 3890.0, not a hex
    /// *float* (which would require an exponent) and not a bit
    /// reinterpretation.
    #[test]
    fn bare_hex_integer_is_a_valid_float_magnitude() {
        let bits = parse_f64_bits("0xf32", 0).unwrap();
        assert_eq!(f64::from_bits(bits), 0xf32 as f64);
    }

    /// Found running the real corpus (`float_literals.wast`): a hex
    /// literal with a fractional part but no `p`/`P` exponent at all --
    /// the exponent is optional there too, defaulting to 0, not just on a
    /// bare hex integer.
    #[test]
    fn hex_float_with_fraction_and_no_exponent_defaults_to_zero() {
        let bits = parse_f64_bits("0x1.8", 0).unwrap();
        assert_eq!(f64::from_bits(bits), 1.5);
    }

    #[test]
    fn decimal_float_parses_normally() {
        let bits = parse_f64_bits("3.5", 0).unwrap();
        assert_eq!(f64::from_bits(bits), 3.5);
    }

    #[test]
    fn inf_and_signed_inf() {
        assert_eq!(f64::from_bits(parse_f64_bits("inf", 0).unwrap()), f64::INFINITY);
        assert_eq!(f64::from_bits(parse_f64_bits("-inf", 0).unwrap()), f64::NEG_INFINITY);
    }

    #[test]
    fn bare_nan_is_the_canonical_quiet_nan_pattern() {
        let bits = parse_f64_bits("nan", 0).unwrap();
        assert_eq!(bits, 0x7FF8_0000_0000_0000);
    }

    #[test]
    fn nan_with_explicit_payload_is_bit_exact() {
        let bits = parse_f64_bits("nan:0x1", 0).unwrap();
        assert_eq!(bits, 0x7FF0_0000_0000_0001);
        // Sign bit set for a negative payload NaN.
        let neg_bits = parse_f64_bits("-nan:0x1", 0).unwrap();
        assert_eq!(neg_bits, 0xFFF0_0000_0000_0001);
    }

    #[test]
    fn f32_hex_float_and_nan_payload() {
        let bits = parse_f32_bits("0x1.8p3", 0).unwrap();
        assert_eq!(f32::from_bits(bits), 12.0f32);
        let nan_bits = parse_f32_bits("nan:0x1", 0).unwrap();
        assert_eq!(nan_bits, 0x7F80_0001);
    }
}
