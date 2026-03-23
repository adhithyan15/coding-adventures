//! IEEE 754 encoding and decoding — converting between Rust `f64` values and
//! our explicit bit-level representation ([`FloatBits`]).
//!
//! # How does a computer store 3.14?
//!
//! When you write `let x: f64 = 3.14;` in Rust, the computer stores it as 64 bits
//! following the IEEE 754 standard. This module converts between Rust's native
//! float representation and our explicit bit-level representation (`FloatBits`).
//!
//! # Encoding: float -> bits
//!
//! For FP32, we use Rust's `f32::to_bits()` which gives us the exact same bit
//! pattern that the hardware uses. For FP16 and BF16, we manually extract the
//! bits because Rust doesn't natively support these formats.
//!
//! # Special values in IEEE 754
//!
//! IEEE 754 reserves certain bit patterns for special values:
//!
//! ```text
//! Exponent      Mantissa    Meaning
//! ----------    --------    -------
//! All 1s        All 0s      +/- Infinity
//! All 1s        Non-zero    NaN (Not a Number)
//! All 0s        All 0s      +/- Zero
//! All 0s        Non-zero    Denormalized number (very small, near zero)
//! Other         Any         Normal number
//! ```

use crate::formats::{FloatFormat, FloatBits, FP32, make_nan, make_inf, make_zero};

// =========================================================================
// Helper: integer <-> bit list conversions
// =========================================================================

/// Converts a non-negative integer to a vector of bits, MSB first.
///
/// This is the fundamental conversion between Rust integers and our bit-level
/// representation.
///
/// # Example
///
/// ```
/// use fp_arithmetic::int_to_bits_msb;
/// let bits = int_to_bits_msb(5, 8);
/// assert_eq!(bits, vec![0, 0, 0, 0, 0, 1, 0, 1]);
/// //                     128 64 32 16  8  4  2  1
/// //                                   4     1  = 5
/// ```
///
/// # How it works
///
/// We check each bit position from MSB to LSB. For each position `i` (counting
/// from `width-1` down to 0), we check if that bit is set using a right-shift
/// and AND with 1.
pub fn int_to_bits_msb(value: u64, width: u32) -> Vec<u8> {
    let mut bits = vec![0u8; width as usize];
    for i in 0..width {
        bits[i as usize] = ((value >> (width - 1 - i)) & 1) as u8;
    }
    bits
}

/// Converts a vector of bits (MSB first) back to a non-negative integer.
///
/// This is the inverse of [`int_to_bits_msb`].
///
/// # Example
///
/// ```
/// use fp_arithmetic::bits_msb_to_int;
/// let value = bits_msb_to_int(&[0, 0, 0, 0, 0, 1, 0, 1]);
/// assert_eq!(value, 5);
/// // Each bit contributes: bit_value * 2^position
/// // 0*128 + 0*64 + 0*32 + 0*16 + 0*8 + 1*4 + 0*2 + 1*1 = 5
/// ```
///
/// # How it works
///
/// Iterate from MSB to LSB. For each bit, shift the accumulator left by 1
/// (multiply by 2) and OR in the new bit.
pub fn bits_msb_to_int(bits: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for &bit in bits {
        result = (result << 1) | (bit as u64);
    }
    result
}

// =========================================================================
// Encoding: Rust f64 -> FloatBits
// =========================================================================

/// Converts a Rust `f64` to its IEEE 754 bit representation in the given format.
///
/// # How FP32 encoding works
///
/// For FP32, we use `f32::to_bits()` which gives us the exact bit pattern
/// that the hardware uses. We then extract the sign, exponent, and mantissa
/// fields using bit shifts and masks.
///
/// # How FP16/BF16 encoding works (manual)
///
/// For FP16 and BF16, Rust doesn't have native support, so we:
///  1. First encode as FP32 (which we know is exact for the hardware)
///  2. Extract the sign, exponent, and mantissa from the FP32 encoding
///  3. Re-encode into the target format, adjusting exponent bias and
///     truncating the mantissa
///
/// # Worked example: encoding 3.14 as FP32
///
/// ```text
/// 3.14 in binary: 11.00100011110101110000101...
/// Normalized:     1.100100011110101110000101... x 2^1
///
/// Sign:     0 (positive)
/// Exponent: 1 + 127 (bias) = 128 = 10000000 in binary
/// Mantissa: 10010001111010111000010 (23 bits after the implicit 1)
///                                    ^-- note: the leading 1 is NOT stored
/// ```
pub fn float_to_bits(value: f64, fmt: FloatFormat) -> FloatBits {
    // --- Handle NaN specially ---
    // Rust has f64::NAN, and IEEE 754 defines NaN as exponent=all-1s,
    // mantissa=non-zero. We use a "quiet NaN" with the MSB of mantissa set.
    if value.is_nan() {
        return make_nan(fmt);
    }

    // --- Handle Infinity ---
    // +Inf and -Inf: exponent=all-1s, mantissa=all-0s.
    if value.is_infinite() {
        let sign = if value < 0.0 { 1 } else { 0 };
        return make_inf(sign, fmt);
    }

    // --- FP32: use f32::to_bits() for hardware-exact encoding ---
    if fmt == FP32 {
        // f32::to_bits() gives us the raw 32-bit IEEE 754 representation.
        let int_bits = (value as f32).to_bits();

        // Extract the three fields using bit shifts and masks:
        //   Bit 31:     sign
        //   Bits 30-23: exponent (8 bits)
        //   Bits 22-0:  mantissa (23 bits)
        let sign = ((int_bits >> 31) & 1) as u8;
        let exp_int = ((int_bits >> 23) & 0xFF) as u64;
        let mant_int = (int_bits & 0x7FFFFF) as u64;

        return FloatBits {
            sign,
            exponent: int_to_bits_msb(exp_int, 8),
            mantissa: int_to_bits_msb(mant_int, 23),
            fmt: FP32,
        };
    }

    // --- FP16 and BF16: manual conversion from FP32 ---
    //
    // Strategy: encode as FP32 first, then convert.
    // This handles all the tricky cases (denormals, rounding) correctly.
    let fp32_bits = float_to_bits(value, FP32);
    let fp32_exp = bits_msb_to_int(&fp32_bits.exponent) as i32;
    let fp32_mant = bits_msb_to_int(&fp32_bits.mantissa) as u64;
    let sign = fp32_bits.sign;

    // --- Handle zero ---
    if fp32_exp == 0 && fp32_mant == 0 {
        return make_zero(sign, fmt);
    }

    // --- Compute the true (unbiased) exponent ---
    let true_exp: i32;
    let full_mantissa: u64;
    if fp32_exp == 0 {
        // Denormal in FP32: true exponent is -126, implicit bit is 0
        true_exp = 1 - FP32.bias; // = -126
        full_mantissa = fp32_mant;
    } else {
        true_exp = fp32_exp - FP32.bias;
        // Normal: full mantissa includes the implicit leading 1
        full_mantissa = (1u64 << FP32.mantissa_bits) | fp32_mant;
    }

    // --- Map to target format ---
    let mut target_exp = true_exp + fmt.bias;
    let max_exp = (1i32 << fmt.exponent_bits) - 1;

    // --- Overflow: exponent too large for target format -> Infinity ---
    if target_exp >= max_exp {
        return make_inf(sign, fmt);
    }

    // --- Normal case: exponent fits in target format ---
    if target_exp > 0 {
        let mut truncated: u64;
        if fmt.mantissa_bits < FP32.mantissa_bits {
            let shift = FP32.mantissa_bits - fmt.mantissa_bits;
            truncated = fp32_mant >> shift;
            // Round-to-nearest-even
            let round_bit = (fp32_mant >> (shift - 1)) & 1;
            let sticky = fp32_mant & ((1u64 << (shift - 1)) - 1);
            if round_bit != 0 && (sticky != 0 || (truncated & 1) != 0) {
                truncated += 1;
                // Rounding overflow
                if truncated >= (1u64 << fmt.mantissa_bits) {
                    truncated = 0;
                    target_exp += 1;
                    if target_exp >= max_exp {
                        return make_inf(sign, fmt);
                    }
                }
            }
        } else {
            truncated = fp32_mant << (fmt.mantissa_bits - FP32.mantissa_bits);
        }

        return FloatBits {
            sign,
            exponent: int_to_bits_msb(target_exp as u64, fmt.exponent_bits),
            mantissa: int_to_bits_msb(truncated, fmt.mantissa_bits),
            fmt,
        };
    }

    // --- Underflow: number is too small for normal representation ---
    // It might still be representable as a denormal in the target format.
    let denorm_shift = 1 - target_exp;

    if denorm_shift > fmt.mantissa_bits as i32 {
        // Too small even for denormal -> flush to zero
        return make_zero(sign, fmt);
    }

    // Shift the full mantissa right to create a denormal
    let total_shift = denorm_shift as u32 + FP32.mantissa_bits - fmt.mantissa_bits;
    let denorm_mant = full_mantissa >> total_shift;

    FloatBits {
        sign,
        exponent: vec![0u8; fmt.exponent_bits as usize],
        mantissa: int_to_bits_msb(
            denorm_mant & ((1u64 << fmt.mantissa_bits) - 1),
            fmt.mantissa_bits,
        ),
        fmt,
    }
}

// =========================================================================
// Decoding: FloatBits -> Rust f64
// =========================================================================

/// Converts an IEEE 754 bit representation back to a Rust `f64`.
///
/// # How decoding works
///
/// For FP32, we reconstruct the 32-bit integer and use `f32::from_bits()` to
/// get the exact Rust float. For FP16/BF16, we manually compute the value using:
///
/// ```text
/// value = (-1)^sign x 2^(exponent - bias) x 1.mantissa
/// ```
pub fn bits_to_float(bits: &FloatBits) -> f64 {
    let exp_int = bits_msb_to_int(&bits.exponent) as i32;
    let mant_int = bits_msb_to_int(&bits.mantissa);
    let max_exp = (1i32 << bits.fmt.exponent_bits) - 1;

    // --- Special values ---

    // NaN: exponent all 1s, mantissa non-zero
    if exp_int == max_exp && mant_int != 0 {
        return f64::NAN;
    }

    // Infinity: exponent all 1s, mantissa all zeros
    if exp_int == max_exp && mant_int == 0 {
        return if bits.sign == 1 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // Zero: exponent all 0s, mantissa all zeros
    if exp_int == 0 && mant_int == 0 {
        return if bits.sign == 1 {
            -0.0_f64
        } else {
            0.0_f64
        };
    }

    // --- For FP32, use f32::from_bits() for exact conversion ---
    if bits.fmt == FP32 {
        let int_bits =
            ((bits.sign as u32) << 31) | ((exp_int as u32) << 23) | (mant_int as u32);
        return f32::from_bits(int_bits) as f64;
    }

    // --- For FP16/BF16, compute the float value manually ---
    let true_exp: i32;
    let mantissa_value: f64;

    if exp_int == 0 {
        // Denormalized: value = (-1)^sign x 2^(1-bias) x 0.mantissa
        true_exp = 1 - bits.fmt.bias;
        mantissa_value = (mant_int as f64) / ((1u64 << bits.fmt.mantissa_bits) as f64);
    } else {
        // Normal: implicit leading 1
        true_exp = exp_int - bits.fmt.bias;
        mantissa_value =
            1.0 + (mant_int as f64) / ((1u64 << bits.fmt.mantissa_bits) as f64);
    }

    let mut result = mantissa_value * 2.0_f64.powi(true_exp);
    if bits.sign == 1 {
        result = -result;
    }

    result
}

// =========================================================================
// Special value detection
// =========================================================================
//
// These functions detect special IEEE 754 values by examining the bit pattern.
// We use simple boolean logic to check bit fields, staying true to the
// "built from gates" philosophy.

/// Checks if all bits in a slice are 1.
///
/// In hardware, this would be a wide AND gate:
///
/// ```text
/// all_ones = AND(bit[0], AND(bit[1], AND(bit[2], ...)))
/// ```
///
/// If ALL bits are 1, the final AND output is 1. If ANY bit is 0, it collapses to 0.
fn all_ones(bits: &[u8]) -> bool {
    bits.iter().all(|&b| b == 1)
}

/// Checks if all bits in a slice are 0.
///
/// In hardware: NOR across all bits.
///
/// ```text
/// any_one = OR(bit[0], OR(bit[1], OR(bit[2], ...)))
/// all_zeros = NOT(any_one)
/// ```
///
/// If ANY bit is 1, the OR chain produces 1, and we return false.
/// If ALL bits are 0, the OR chain produces 0, and we return true.
fn all_zeros(bits: &[u8]) -> bool {
    bits.iter().all(|&b| b == 0)
}

/// Checks if a `FloatBits` represents NaN (Not a Number).
///
/// NaN is defined as: exponent = all 1s AND mantissa != all 0s.
///
/// In IEEE 754, NaN is the result of undefined operations like:
///
/// ```text
/// 0 / 0, Inf - Inf, sqrt(-1)
/// ```
///
/// There are two types of NaN:
///   - Quiet NaN (qNaN): mantissa MSB = 1, propagates silently
///   - Signaling NaN (sNaN): mantissa MSB = 0, raises exception
///
/// We don't distinguish between them here.
pub fn is_nan(bits: &FloatBits) -> bool {
    all_ones(&bits.exponent) && !all_zeros(&bits.mantissa)
}

/// Checks if a `FloatBits` represents Infinity (+Inf or -Inf).
///
/// Infinity is defined as: exponent = all 1s AND mantissa = all 0s.
///
/// IEEE 754 uses Infinity to represent overflow results:
///
/// ```text
/// 1e38 * 10 = +Inf (in FP32)
/// -1.0 / 0.0 = -Inf
/// ```
pub fn is_inf(bits: &FloatBits) -> bool {
    all_ones(&bits.exponent) && all_zeros(&bits.mantissa)
}

/// Checks if a `FloatBits` represents zero (+0 or -0).
///
/// Zero is defined as: exponent = all 0s AND mantissa = all 0s.
///
/// IEEE 754 has both +0 and -0. They compare equal (`0.0 == -0.0` in Rust),
/// but they are different bit patterns. Having -0 is important for preserving
/// the sign through operations like `1.0 / -Inf = -0`.
pub fn is_zero(bits: &FloatBits) -> bool {
    all_zeros(&bits.exponent) && all_zeros(&bits.mantissa)
}

/// Checks if a `FloatBits` represents a denormalized (subnormal) number.
///
/// Denormalized is defined as: exponent = all 0s AND mantissa != all 0s.
///
/// # What are denormalized numbers?
///
/// Normal IEEE 754 numbers have an implicit leading 1: the value is `1.mantissa`.
/// But what about very small numbers close to zero? The smallest normal FP32
/// number is about 1.18e-38. Without denormals, the next smaller value would
/// be 0 — a sudden jump called "the underflow gap."
///
/// Denormalized numbers fill this gap. When the exponent is all zeros, the
/// implicit bit becomes 0 instead of 1, and the true exponent is fixed at
/// `(1 - bias)`. This allows gradual underflow: numbers smoothly approach zero.
///
/// ```text
/// Normal:     1.mantissa x 2^(exp-bias)     (implicit 1)
/// Denormal:   0.mantissa x 2^(1-bias)       (implicit 0)
/// ```
pub fn is_denormalized(bits: &FloatBits) -> bool {
    all_zeros(&bits.exponent) && !all_zeros(&bits.mantissa)
}

/// Returns the position of the highest set bit + 1, like Python's `int.bit_length()`.
///
/// For example: `bit_length(5) = 3`, `bit_length(1) = 1`, `bit_length(0) = 0`.
/// This is essential for normalization: we need to know where the leading 1 is.
pub(crate) fn bit_length(v: u64) -> u32 {
    if v == 0 {
        0
    } else {
        64 - v.leading_zeros()
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{FP16, BF16};

    #[test]
    fn test_int_to_bits_msb() {
        assert_eq!(int_to_bits_msb(5, 8), vec![0, 0, 0, 0, 0, 1, 0, 1]);
        assert_eq!(int_to_bits_msb(0, 4), vec![0, 0, 0, 0]);
        assert_eq!(int_to_bits_msb(15, 4), vec![1, 1, 1, 1]);
        assert_eq!(int_to_bits_msb(1, 1), vec![1]);
        assert_eq!(int_to_bits_msb(127, 8), vec![0, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn test_bits_msb_to_int() {
        assert_eq!(bits_msb_to_int(&[0, 0, 0, 0, 0, 1, 0, 1]), 5);
        assert_eq!(bits_msb_to_int(&[0, 0, 0, 0]), 0);
        assert_eq!(bits_msb_to_int(&[1, 1, 1, 1]), 15);
        assert_eq!(bits_msb_to_int(&[1]), 1);
        assert_eq!(bits_msb_to_int(&[0, 1, 1, 1, 1, 1, 1, 1]), 127);
    }

    /// Verify that encoding and decoding produces the original value (round-trip).
    #[test]
    fn test_round_trip() {
        let values = [0.0_f64, 1.0, -1.0, 3.14, -3.14, 0.5, 100.0, 0.001];
        for &v in &values {
            let bits = float_to_bits(v, FP32);
            let got = bits_to_float(&bits);
            assert_eq!(
                got as f32, v as f32,
                "FP32 round-trip({v}): got {got}"
            );
        }
    }

    /// Test encoding of special IEEE 754 values.
    #[test]
    fn test_float_to_bits_special_values() {
        // Positive zero
        let bits = float_to_bits(0.0, FP32);
        assert_eq!(bits.sign, 0);
        assert!(is_zero(&bits));

        // Negative zero
        let bits = float_to_bits(-0.0_f64, FP32);
        assert_eq!(bits.sign, 1);
        assert!(is_zero(&bits));

        // Positive infinity
        let bits = float_to_bits(f64::INFINITY, FP32);
        assert_eq!(bits.sign, 0);
        assert!(is_inf(&bits));

        // Negative infinity
        let bits = float_to_bits(f64::NEG_INFINITY, FP32);
        assert_eq!(bits.sign, 1);
        assert!(is_inf(&bits));

        // NaN
        let bits = float_to_bits(f64::NAN, FP32);
        assert!(is_nan(&bits));
    }

    /// Test encoding of specific well-known values.
    #[test]
    fn test_float_to_bits_known_values() {
        // 1.0 in FP32: sign=0, exponent=01111111 (127), mantissa=all zeros
        let bits = float_to_bits(1.0, FP32);
        assert_eq!(bits.sign, 0);
        assert_eq!(bits_msb_to_int(&bits.exponent), 127);
        assert_eq!(bits_msb_to_int(&bits.mantissa), 0);

        // -2.0 in FP32: sign=1, exponent=128, mantissa=0
        let bits = float_to_bits(-2.0, FP32);
        assert_eq!(bits.sign, 1);
        assert_eq!(bits_msb_to_int(&bits.exponent), 128);
    }

    /// Test decoding of special values.
    #[test]
    fn test_bits_to_float_special_values() {
        // NaN
        assert!(bits_to_float(&make_nan(FP32)).is_nan());

        // +Inf
        let v = bits_to_float(&make_inf(0, FP32));
        assert!(v.is_infinite() && v > 0.0);

        // -Inf
        let v = bits_to_float(&make_inf(1, FP32));
        assert!(v.is_infinite() && v < 0.0);

        // +0
        let v = bits_to_float(&make_zero(0, FP32));
        assert_eq!(v, 0.0);
        assert!(!v.is_sign_negative());

        // -0
        let v = bits_to_float(&make_zero(1, FP32));
        assert_eq!(v, 0.0);
        assert!(v.is_sign_negative());
    }

    #[test]
    fn test_is_nan() {
        assert!(is_nan(&float_to_bits(f64::NAN, FP32)));
        assert!(!is_nan(&float_to_bits(1.0, FP32)));
        assert!(!is_nan(&float_to_bits(f64::INFINITY, FP32)));
    }

    #[test]
    fn test_is_inf() {
        assert!(is_inf(&float_to_bits(f64::INFINITY, FP32)));
        assert!(is_inf(&float_to_bits(f64::NEG_INFINITY, FP32)));
        assert!(!is_inf(&float_to_bits(1.0, FP32)));
    }

    #[test]
    fn test_is_zero() {
        assert!(is_zero(&float_to_bits(0.0, FP32)));
        assert!(is_zero(&float_to_bits(-0.0_f64, FP32)));
        assert!(!is_zero(&float_to_bits(1.0, FP32)));
    }

    #[test]
    fn test_is_denormalized() {
        // Create the smallest positive denormal in FP32
        let mut mantissa = vec![0u8; 23];
        mantissa[22] = 1;
        let tiny = FloatBits {
            sign: 0,
            exponent: vec![0u8; 8],
            mantissa,
            fmt: FP32,
        };
        assert!(is_denormalized(&tiny));
        assert!(!is_denormalized(&float_to_bits(1.0, FP32)));
        assert!(!is_denormalized(&float_to_bits(0.0, FP32)));
    }

    /// Test FP16 encoding and decoding.
    #[test]
    fn test_fp16_encoding() {
        let test_cases: Vec<(&str, f64)> = vec![
            ("1.0", 1.0),
            ("-1.0", -1.0),
            ("0.5", 0.5),
            ("0.0", 0.0),
            ("+Inf", f64::INFINITY),
            ("-Inf", f64::NEG_INFINITY),
            ("NaN", f64::NAN),
        ];

        for (name, value) in test_cases {
            let bits = float_to_bits(value, FP16);
            let got = bits_to_float(&bits);
            if value.is_nan() {
                assert!(got.is_nan(), "FP16 NaN: got {got}");
            } else if value.is_infinite() {
                assert!(got.is_infinite(), "FP16 Inf: got {got}");
            } else {
                assert_eq!(got, value, "FP16({name}): got {got}");
            }
        }
    }

    /// Test BF16 encoding and decoding.
    #[test]
    fn test_bf16_encoding() {
        let test_cases: Vec<(&str, f64)> = vec![
            ("1.0", 1.0),
            ("-1.0", -1.0),
            ("0.0", 0.0),
            ("+Inf", f64::INFINITY),
            ("NaN", f64::NAN),
        ];

        for (name, value) in test_cases {
            let bits = float_to_bits(value, BF16);
            let got = bits_to_float(&bits);
            if value.is_nan() {
                assert!(got.is_nan(), "BF16 NaN: got {got}");
            } else if value.is_infinite() {
                assert!(got.is_infinite(), "BF16 Inf: got {got}");
            } else {
                assert_eq!(got, value, "BF16({name}): got {got}");
            }
        }
    }

    /// Test that large FP32 values overflow to Inf in FP16.
    #[test]
    fn test_fp16_overflow() {
        let bits = float_to_bits(100000.0, FP16);
        assert!(is_inf(&bits), "100000.0 in FP16 should overflow to Inf");
    }

    /// Test that very small values underflow to zero in FP16.
    #[test]
    fn test_fp16_underflow() {
        let bits = float_to_bits(1e-20, FP16);
        assert!(is_zero(&bits), "1e-20 in FP16 should underflow to zero");
    }

    #[test]
    fn test_bit_length() {
        assert_eq!(bit_length(0), 0);
        assert_eq!(bit_length(1), 1);
        assert_eq!(bit_length(2), 2);
        assert_eq!(bit_length(3), 2);
        assert_eq!(bit_length(4), 3);
        assert_eq!(bit_length(5), 3);
        assert_eq!(bit_length(255), 8);
        assert_eq!(bit_length(256), 9);
    }

    /// Test special value detectors with FP16.
    #[test]
    fn test_fp16_special_values_detection() {
        assert!(is_nan(&float_to_bits(f64::NAN, FP16)));
        assert!(is_inf(&float_to_bits(f64::INFINITY, FP16)));
        assert!(is_zero(&float_to_bits(0.0, FP16)));
    }

    /// Test special value detectors with BF16.
    #[test]
    fn test_bf16_special_values_detection() {
        assert!(is_nan(&float_to_bits(f64::NAN, BF16)));
        assert!(is_inf(&float_to_bits(f64::NEG_INFINITY, BF16)));
    }
}
