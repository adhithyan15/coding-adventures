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

/// Parse a `u32`-context integer literal -- table/memory LIMITS
/// (`min`/`max`), decimal or `0x`-hex, `_`-separated (task #99: the real
/// testsuite's own `table.wast` uses hex limits like `0xffff_ffff`, which
/// `parse_limits`'s old digit-only atom filter silently dropped instead of
/// parsing). Unlike [`parse_i32`], the WAT grammar never signs a limit (no
/// signed/unsigned dual-spelling to resolve), so a leading `-` is a real
/// error here, not a valid alternate encoding of a large unsigned value.
pub fn parse_u32(text: &str, pos: usize) -> Result<u32, WastParseError> {
    let (neg, mag) = parse_int_magnitude(text, pos)?;
    if neg || mag > u32::MAX as u128 {
        return Err(WastParseError::InvalidNumericLiteralForType {
            pos,
            text: text.to_string(),
            ty: "u32 limit",
        });
    }
    Ok(mag as u32)
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

/// Split a hex-float's mantissa text into hex digits (int part then frac
/// part concatenated, MSB-first) plus the binary exponent that applies to
/// the LAST digit's LSB: `value = digits_as_one_big_integer * 2^base_exp2`
/// (each hex digit IS exactly 4 bits, so no decimal-to-binary conversion
/// error is possible here -- unlike parsing a DECIMAL float exactly, which
/// needs bignum/Dragon4-style machinery, a hex float's bits are already
/// exact; the only real work, done by [`round_hex_mantissa`], is finding
/// where to CUT and how to ROUND for a given target width). Shared by both
/// the `f64` and `f32` parsing paths so hex-digit parsing/validation isn't
/// duplicated between them.
fn split_hex_float(hex: &str, text: &str, pos: usize) -> Result<(Vec<u32>, i64), WastParseError> {
    // WAT's `p`/`P` exponent is OPTIONAL, not just on a bare hex integer
    // (`f32.const 0xf32` = 3890.0) but also on a hex literal with a
    // fractional part (`0xa0_ff.f141_a59a` with no exponent at all) --
    // both default to exponent 0, only the mantissa parsing differs based
    // on whether a `.` is present.
    let (mantissa_str, exp_str) = hex.split_once(['p', 'P']).unwrap_or((hex, "0"));
    let (int_part, frac_part) = match mantissa_str.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa_str, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() });
    }
    let mut digits: Vec<u32> = Vec::with_capacity(int_part.len() + frac_part.len());
    for c in int_part.chars().chain(frac_part.chars()) {
        digits.push(c.to_digit(16).ok_or(WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?);
    }
    let exponent: i64 = exp_str
        .parse()
        .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?;
    // Each hex digit after the point shifts the true value right by
    // exactly 4 bits (16^-1 = 2^-4). Saturating, not `-`: `exponent` is a
    // fully attacker-controlled `i64` from the literal's `p<exponent>`
    // text (e.g. `p-9223372036854775808`) -- with any non-empty
    // `frac_part`, plain subtraction near `i64::MIN` underflow-panics
    // (security review finding, task #80). Saturating to `i64::MIN` is
    // semantically correct: `round_hex_mantissa`'s underflow guard flushes
    // any such extreme `base_exp2` to 0.0 regardless of how far past the
    // real minimum it saturates.
    let base_exp2 = exponent.saturating_sub(4i64.saturating_mul(frac_part.len() as i64));
    Ok((digits, base_exp2))
}

/// Parse a plain (non-NaN, non-inf) WAT `f64` literal — decimal or hex — to
/// its bit pattern. Hex floats (`0x1.8p3`) are computed bit-exact via
/// [`round_hex_mantissa`] -- see that function's own doc comment for why
/// naive digit-by-digit `f64` accumulation (this function's OWN approach
/// until task #80) is NOT good enough: it double-rounds and can produce
/// the wrong nearest value for mantissas wider than f64's own 53-bit
/// significand.
fn parse_float_magnitude_f64_bits(text: &str, pos: usize) -> Result<u64, WastParseError> {
    let cleaned = strip_underscores(text);
    if let Some(hex) = cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        let (digits, base_exp2) = split_hex_float(hex, text, pos)?;
        let (exp_field, mantissa) = round_hex_mantissa(&digits, base_exp2, 52, 1023);
        Ok((exp_field << 52) | mantissa)
    } else {
        Ok(cleaned
            .parse::<f64>()
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?
            .to_bits())
    }
}

/// As [`parse_float_magnitude_f64_bits`], but rounds directly to `f32`'s
/// 32-bit layout instead of going through an `f64` intermediate and
/// narrowing. This is NOT just a style choice: rounding to `f64` (53-bit
/// significand) and then narrowing to `f32` (24-bit) is *double rounding*,
/// which is not always equivalent to rounding the exact value to `f32`
/// once directly -- a value can sit close enough to an `f32` rounding
/// boundary that the intermediate `f64` rounding already commits to the
/// wrong side before the narrowing cast ever runs. The real corpus
/// (`const.wast`) has literals exercising exactly this: e.g. an
/// over-precision hex mantissa whose `f32.const` result was still wrong
/// even after [`round_hex_mantissa`] made the `f64` path bit-exact,
/// because the SECOND rounding step (the narrowing `as f32`) was the one
/// actually producing the wrong answer.
fn parse_float_magnitude_f32_bits(text: &str, pos: usize) -> Result<u32, WastParseError> {
    let cleaned = strip_underscores(text);
    if let Some(hex) = cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        let (digits, base_exp2) = split_hex_float(hex, text, pos)?;
        let (exp_field, mantissa) = round_hex_mantissa(&digits, base_exp2, 23, 127);
        Ok(((exp_field as u32) << 23) | mantissa as u32)
    } else {
        Ok(cleaned
            .parse::<f32>()
            .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })?
            .to_bits())
    }
}

/// Round an arbitrary-length hex-float mantissa to the nearest
/// representable value of an IEEE-754 binary format, correctly
/// (round-to-nearest, ties-to-even) -- unlike naive digit-by-digit `f64`
/// accumulation, which performs a SEPARATE rounding step at every one of
/// the (potentially dozens of) hex digits instead of one final rounding of
/// the true mathematical value. That "double rounding" is invisible for
/// short literals (nothing to round away, or the accumulated error
/// happens not to cross a rounding boundary), but produces a genuinely
/// WRONG nearest-value answer for the real spec testsuite's own
/// deliberately-crafted extreme-precision edge cases -- e.g.
/// `simd_const.wast`'s `+0x1.000000000000080000000000p-600` (a value
/// sitting EXACTLY halfway between two representable `f64`s, testing that
/// ties round to even) and `+0x1.000000000000080000000001p-600` (one ULP
/// past halfway, must round up) — found via real conformance grading
/// (task #78/#80), not a hypothetical.
///
/// `digits`/`base_exp2` are as produced by [`split_hex_float`]:
/// `value = digits_as_one_big_integer * 2^base_exp2`. `mantissa_bits` and
/// `exp_bias` parameterize the target IEEE-754 format -- `(52, 1023)` for
/// `f64`, `(23, 127)` for `f32` -- so both callers share one correctly-
/// rounding implementation instead of one being a narrowing cast of the
/// other's (already-rounded) result, which would just reintroduce the
/// same double-rounding problem this function exists to avoid.
///
/// Algorithm: find the leading (most significant) `1` bit of the digit
/// sequence (skipping leading zero nibbles), take the next `mantissa_bits`
/// bits as the explicit mantissa (zero-padded on the right if there
/// aren't enough), then round using the classic guard/round/sticky rule
/// on whatever bits remain -- if the guard bit is 0, truncate; if it's 1
/// and any lower bit is set (sticky), round up; if it's 1 and every lower
/// bit is 0 (an EXACT tie), round to EVEN (round up only if that makes the
/// kept mantissa's LSB 0).
///
/// Returns `(exp_field, mantissa)` as raw bit-field values ready for the
/// caller to pack (`(exp_field << mantissa_bits) | mantissa`) -- already
/// clamped to a subnormal (`exp_field == 0`) or infinity (`exp_field` at
/// the reserved all-ones value, `mantissa == 0`) as appropriate, so
/// callers never need a separate overflow/underflow check of their own.
fn round_hex_mantissa(digits: &[u32], base_exp2: i64, mantissa_bits: i64, exp_bias: i64) -> (u64, u64) {
    // IEEE-754: the smallest normal exponent is `1 - bias` (e.g. -1022 for
    // f64, -126 for f32); the largest normal exponent is `bias` itself
    // (e.g. 1023 for f64, 127 for f32); the reserved (inf/NaN) exponent
    // field is `2*bias + 1`.
    let min_normal_exp = 1 - exp_bias;
    let reserved_field = (2 * exp_bias + 1) as u64;

    let Some(first) = digits.iter().position(|&d| d != 0) else {
        return (0, 0); // every digit is zero -- the literal denotes 0.0
    };
    let sig = &digits[first..];
    let n = sig.len() as i64;

    // `sig[0]` is 1..=15 (nonzero by construction), so it occupies 1..=4
    // bits -- e.g. `0x1` needs 1 bit, `0x8` needs 4.
    let lead_bits = 32 - sig[0].leading_zeros();
    let bitlen = (n - 1) * 4 + lead_bits as i64;

    // Bit `k` of the significant digit sequence, 0 = its own LSB.
    // Reading past either end (a nibble that doesn't exist, or a
    // fractional bit beyond the last digit) is a legitimate "not present"
    // case, not a bug -- it means "treat as 0", exactly right for both
    // zero-padding a short mantissa and for the sticky-bit scan below.
    let bit_at = |k: i64| -> u64 {
        if k < 0 {
            return 0;
        }
        let nibble_from_end = k / 4;
        if nibble_from_end >= n {
            return 0;
        }
        let nibble = sig[(n - 1 - nibble_from_end) as usize];
        ((nibble >> (k % 4)) & 1) as u64
    };

    // `value = 1.<mantissa_bits> * 2^e` once normalized -- `e` is the
    // binary exponent of the leading bit (bit `bitlen - 1`) after
    // `base_exp2` shifts the whole digit sequence into place. Saturating,
    // not `+`/`-`: `base_exp2` can already be `i64::MAX`/`i64::MIN`-
    // adjacent from an attacker-controlled `p<exponent>` (e.g.
    // `p9223372036854775807`), and plain arithmetic here overflow-panics
    // BEFORE either guard below gets a chance to run (security review
    // finding, task #80) -- the guards below only protect the guard/
    // sticky bit-scan from an unbounded range, they can't help if `e`
    // itself never finishes computing. Saturating is semantically
    // correct: an `e` this extreme is always far outside the guards'
    // `[min_normal_exp - mantissa_bits - 1, exp_bias]` window regardless
    // of exactly where it saturates, so the guards still fire correctly.
    let e = base_exp2.saturating_add(bitlen).saturating_sub(1);

    // Exponents below `min_normal_exp` have no implicit leading 1 -- the
    // value is a SUBNORMAL, stored as `0.<mantissa_bits> * 2^min_normal_exp`
    // instead of `1.<mantissa_bits> * 2^e`. Real corpus literals exercise
    // this directly (e.g. `f64.wast`'s `0x0.0000000000001p-1022`, the
    // smallest positive subnormal `f64`) -- flushing these to 0.0 (this
    // function's first, wrong attempt at task #80) is a real,
    // corpus-confirmed bug, not a hypothetical edge case.
    //
    // Below `min_normal_exp - mantissa_bits - 1` (half a ULP under the
    // smallest subnormal) the result is unrepresentable and must flush to
    // 0.0 regardless of rounding -- returning early here, BEFORE the
    // guard/sticky scan below, matters for more than tidiness: that
    // scan's range is driven by `e` (via `base_exp2`), so an adversarial
    // literal with a huge negative `p<exponent>` (input size does not
    // bound `e`, unlike the digit count) would otherwise make the scan
    // iterate a huge or effectively unbounded number of times -- a real
    // DoS vector, not caught by any existing depth/length guard elsewhere
    // in this parser. The symmetric positive-overflow case (`e >
    // exp_bias`) does NOT share this hazard (the scan range collapses to
    // a small, digit-count-bound span there), but is handled early anyway
    // for clarity and to skip needless work.
    if e < min_normal_exp - mantissa_bits - 1 {
        return (0, 0);
    }
    if e > exp_bias {
        return (reserved_field, 0);
    }

    // Reference exponent for the kept mantissa field's top bit position
    // (`ref_exp - 1`): for a normal number this sits directly below the
    // (unstored, implicit) leading 1 at exponent `e`; for a subnormal
    // number there is no implicit bit, so the field is pinned to the
    // smallest normal exponent instead of tracking `e` further down --
    // the actual leading 1 then shows up as an explicit bit somewhere
    // inside the mantissa field, preceded by real leading zeros.
    let ref_exp = e.max(min_normal_exp);
    let is_normal = e >= min_normal_exp;

    let mut mantissa: u64 = 0;
    for i in 0..mantissa_bits {
        mantissa = (mantissa << 1) | bit_at(ref_exp - 1 - i - base_exp2);
    }
    let guard = bit_at(ref_exp - 1 - mantissa_bits - base_exp2);
    let sticky = (0..=(ref_exp - 2 - mantissa_bits - base_exp2)).rev().any(|k| bit_at(k) != 0);
    let round_up = guard == 1 && (sticky || (mantissa & 1) == 1);

    let mut exp_field: i64 = if is_normal { e + exp_bias } else { 0 };
    if round_up {
        mantissa += 1;
        if mantissa == (1u64 << mantissa_bits) {
            // Mantissa overflow: for a normal number this is the usual
            // carry into the (unstored) implicit bit, bumping the
            // exponent. For a subnormal number, this is a round-UP that
            // crosses the subnormal/normal boundary -- e.g. the largest
            // subnormal rounding up becomes the SMALLEST normal (exponent
            // field 0 -> 1) -- exactly the same "mantissa wraps to 0,
            // exponent field +1" step handles both, since a subnormal's
            // exponent field starts at 0.
            mantissa = 0;
            exp_field += 1;
        }
    }

    if exp_field as u64 >= reserved_field {
        return (reserved_field, 0);
    }
    (exp_field as u64, mantissa)
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
        parse_float_magnitude_f64_bits(rest, pos)?
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
        parse_float_magnitude_f32_bits(rest, pos)?
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

    /// Task #80: real corpus failures (`simd_const.wast`, vendored in PR
    /// #11840) exposed that the OLD digit-by-digit `f64` accumulation
    /// double-rounds instead of rounding the true mathematical value once.
    /// These two literals differ by exactly 1 in their last hex digit and
    /// sit on either side of an exact tie between two adjacent `f64`s --
    /// the first must round DOWN (ties-to-even: the kept bit is already
    /// even) and the second must round UP (one ULP past the tie). Expected
    /// bit patterns cross-checked against Python's `float.fromhex` (itself
    /// a correctly-rounding, spec-conformant hex-float parser).
    #[test]
    fn hex_float_overprecise_mantissa_ties_to_even_rounds_down() {
        let bits = parse_f64_bits("0x1.000000000000080000000000p-600", 0).unwrap();
        assert_eq!(bits, 0x1a70_0000_0000_0000);
    }

    #[test]
    fn hex_float_overprecise_mantissa_just_past_tie_rounds_up() {
        let bits = parse_f64_bits("0x1.000000000000080000000001p-600", 0).unwrap();
        assert_eq!(bits, 0x1a70_0000_0000_0001);
    }

    /// A mantissa with more than 52 significant bits that is NOT anywhere
    /// near a rounding tie -- guards against an off-by-one in the
    /// guard/sticky split producing the wrong answer in the common case,
    /// not just at exact ties.
    #[test]
    fn hex_float_overprecise_mantissa_clearly_rounds_up() {
        // 0x1.fffffffffffff8...f p0 -- every bit set past the 52 kept
        // explicit bits, so this must round up into the next power of two.
        let bits = parse_f64_bits("0x1.ffffffffffffff0000000000p0", 0).unwrap();
        assert_eq!(f64::from_bits(bits), 2.0);
    }

    /// Task #80 follow-up: the first correct-rounding rewrite fixed the
    /// over-precision tie-break bug but introduced a NEW regression --
    /// flushing every subnormal (denormal) literal to 0.0, confirmed via
    /// the real corpus (`f64.wast` regressed 2500/2500 -> 2420/2500 passing
    /// when this was caught). `0x0.0000000000001p-1022` is the smallest
    /// positive subnormal `f64` (mantissa 1, exponent field 0).
    #[test]
    fn hex_float_smallest_positive_subnormal_is_not_flushed_to_zero() {
        assert_eq!(parse_f64_bits("0x0.0000000000001p-1022", 0).unwrap(), 0x1);
        assert_eq!(parse_f64_bits("-0x0.0000000000001p-1022", 0).unwrap(), 0x8000_0000_0000_0001);
    }

    #[test]
    fn hex_float_smallest_positive_normal_is_the_subnormal_normal_boundary() {
        assert_eq!(parse_f64_bits("0x1p-1022", 0).unwrap(), 0x0010_0000_0000_0000);
    }

    #[test]
    fn hex_float_largest_finite_value_does_not_overflow_to_infinity() {
        let bits = parse_f64_bits("0x1.fffffffffffffp+1023", 0).unwrap();
        assert_eq!(bits, 0x7fef_ffff_ffff_ffff);
        assert!(f64::from_bits(bits).is_finite());
    }

    /// A magnitude genuinely too small to represent (below half a ULP
    /// under the smallest subnormal) must flush to (signed) zero, not
    /// panic or hang -- this also exercises the DoS-guard early return
    /// for an extreme negative exponent that a naive bit-scan loop would
    /// otherwise iterate over unboundedly.
    #[test]
    fn hex_float_below_smallest_subnormal_flushes_to_signed_zero() {
        assert_eq!(parse_f64_bits("0x1p-2000", 0).unwrap(), 0);
        assert_eq!(parse_f64_bits("-0x1p-2000", 0).unwrap(), 0x8000_0000_0000_0000);
    }

    /// A magnitude genuinely too large to represent must saturate to
    /// (signed) infinity, not panic.
    #[test]
    fn hex_float_above_largest_finite_value_overflows_to_signed_infinity() {
        assert_eq!(parse_f64_bits("0x1p+2000", 0).unwrap(), f64::INFINITY.to_bits());
        assert_eq!(parse_f64_bits("-0x1p+2000", 0).unwrap(), f64::NEG_INFINITY.to_bits());
    }

    /// Security review regression (task #80): `p<exponent>` is a fully
    /// attacker-controlled `i64` (this parser consumes untrusted `.wast`
    /// source) -- `p9223372036854775807` (`i64::MAX`) used to
    /// overflow-panic computing `e = base_exp2 + bitlen - 1` in
    /// `round_hex_mantissa`, crashing the process on a ~25-byte literal,
    /// BEFORE the overflow/underflow guards in that function ever got a
    /// chance to run. Must saturate and flush to infinity, not panic, on
    /// a debug build (`overflow-checks = true`).
    #[test]
    fn hex_float_extreme_positive_exponent_saturates_instead_of_overflow_panicking() {
        assert_eq!(parse_f64_bits("0x1p9223372036854775807", 0).unwrap(), f64::INFINITY.to_bits());
        assert_eq!(parse_f32_bits("0x1p9223372036854775807", 0).unwrap(), f32::INFINITY.to_bits());
    }

    /// Security review regression (task #80): the mirror case --
    /// `p-9223372036854775808` (`i64::MIN`) combined with any non-empty
    /// fractional part used to underflow-panic in `split_hex_float`'s
    /// `base_exp2 = exponent - 4 * frac_part.len()`, again crashing on a
    /// short, trivially crafted literal. Must saturate and flush to
    /// (signed) zero, not panic.
    #[test]
    fn hex_float_extreme_negative_exponent_with_fraction_saturates_instead_of_underflow_panicking() {
        assert_eq!(parse_f64_bits("0x1.5p-9223372036854775808", 0).unwrap(), 0);
        assert_eq!(parse_f64_bits("-0x1.5p-9223372036854775808", 0).unwrap(), 0x8000_0000_0000_0000);
        assert_eq!(parse_f32_bits("0x1.5p-9223372036854775808", 0).unwrap(), 0);
    }

    #[test]
    fn f32_hex_float_and_nan_payload() {
        let bits = parse_f32_bits("0x1.8p3", 0).unwrap();
        assert_eq!(f32::from_bits(bits), 12.0f32);
        let nan_bits = parse_f32_bits("nan:0x1", 0).unwrap();
        assert_eq!(nan_bits, 0x7F80_0001);
    }

    /// Task #80 follow-up: fixing `f64` rounding alone wasn't enough --
    /// `parse_f32_bits` used to round to `f64` first and narrow via `as
    /// f32`, which is DOUBLE rounding. This literal is a hand-constructed
    /// classic double-rounding failure case: its f32 mantissa bits look
    /// like an exact halfway tie (round-to-even would keep them
    /// unchanged, since the kept bit is even), but there's a nonzero bit
    /// further out (beyond f32's own guard/sticky region) that makes the
    /// TRUE value strictly greater than the tie point -- direct rounding
    /// must round up. Going through `f64` first erases that information:
    /// `f64` itself hits an EXACT tie at ITS boundary and rounds to even
    /// (down, truncating), which then narrows to f32 as a clean value
    /// with no guard bit set at all -- producing 1.0 instead of the
    /// correct `1.0 + 2^-23`. Verified independently via exact rational
    /// arithmetic (Python `fractions.Fraction`), not just against this
    /// crate's own prior behavior.
    #[test]
    fn f32_over_precision_mantissa_is_rounded_directly_not_via_an_f64_intermediate() {
        let bits = parse_f32_bits("0x1.00000100000008p0", 0).unwrap();
        assert_eq!(bits, 0x3f80_0001, "must round up directly, not double-round down to 1.0 via f64");
        assert_eq!(f32::from_bits(bits), 1.0f32 + 2f32.powi(-23));
    }
}
