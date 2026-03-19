//! Floating-point addition and subtraction — built from first principles.
//!
//! # How FP addition works at the hardware level
//!
//! Adding two floating-point numbers is surprisingly complex compared to integer
//! addition. The core difficulty is that the two numbers might have very different
//! exponents, so their mantissas are "misaligned" and must be shifted before they
//! can be added.
//!
//! Consider adding 1.5 + 0.125 in decimal scientific notation:
//!
//! ```text
//! 1.5 x 10^0  +  1.25 x 10^-1
//! ```
//!
//! You can't just add 1.5 + 1.25 because they have different exponents. First,
//! you align them to the same exponent:
//!
//! ```text
//! 1.5   x 10^0
//! 0.125 x 10^0   (shifted 1.25 right by 1 decimal place)
//! -------------
//! 1.625 x 10^0
//! ```
//!
//! Binary FP addition follows the exact same principle, but with binary mantissas
//! and power-of-2 exponents.
//!
//! # The five steps of FP addition
//!
//! ```text
//! Step 1: Compare exponents
//!         Subtract exponents to find the difference.
//!         The number with the smaller exponent gets shifted.
//!
//! Step 2: Align mantissas
//!         Shift the smaller number's mantissa right by the exponent
//!         difference. This is like converting 0.125 to line up with 1.5.
//!
//! Step 3: Add or subtract mantissas
//!         If signs are the same: add mantissas
//!         If signs differ: subtract the smaller from the larger
//!
//! Step 4: Normalize
//!         The result might not be in 1.xxx form. Adjust:
//!         - If overflow (10.xxx): shift right, increment exponent
//!         - If underflow (0.0xxx): shift left, decrement exponent
//!
//! Step 5: Round
//!         The result might have more bits than the format allows.
//!         Round to fit, using "round to nearest even" (banker's rounding).
//! ```

use crate::formats::{FloatBits, make_nan, make_inf, make_zero};
use crate::ieee754::{bits_msb_to_int, int_to_bits_msb, is_nan, is_inf, is_zero, bit_length};

/// Adds two floating-point numbers using the IEEE 754 algorithm.
///
/// This implements the full addition algorithm:
///  1. Handle special cases (NaN, Inf, Zero)
///  2. Compare exponents
///  3. Align mantissas
///  4. Add/subtract mantissas
///  5. Normalize result
///  6. Round to nearest even
///
/// # Worked example: 1.5 + 0.25 in FP32
///
/// ```text
/// 1.5 = 1.1 x 2^0    -> exp=127, mant=10000...0
/// 0.25 = 1.0 x 2^-2   -> exp=125, mant=00000...0
///
/// Step 1: exp_diff = 127 - 125 = 2 (b has smaller exponent)
/// Step 2: Shift b's mantissa right by 2:
///         1.10000...0  (a, with implicit 1)
///         0.01000...0  (b, shifted right by 2)
/// Step 3: Add:  1.10000...0 + 0.01000...0 = 1.11000...0
/// Step 4: Already normalized (starts with 1.)
/// Step 5: No rounding needed (exact)
/// Result: 1.11 x 2^0 = 1.75 (correct!)
/// ```
pub fn fp_add(a: &FloatBits, b: &FloatBits) -> FloatBits {
    let fmt = a.fmt;

    // ===================================================================
    // Step 0: Handle special cases
    // ===================================================================
    // IEEE 754 defines strict rules for special values:
    //   NaN + anything = NaN
    //   Inf + (-Inf) = NaN
    //   Inf + x = Inf (for finite x)
    //   0 + x = x

    // NaN propagation: any NaN input produces NaN output
    if is_nan(a) || is_nan(b) {
        return make_nan(fmt);
    }

    // Infinity handling
    let a_inf = is_inf(a);
    let b_inf = is_inf(b);
    if a_inf && b_inf {
        if a.sign == b.sign {
            return make_inf(a.sign, fmt);
        }
        // Inf + (-Inf) = NaN
        return make_nan(fmt);
    }
    if a_inf {
        return a.clone();
    }
    if b_inf {
        return b.clone();
    }

    // Zero handling
    let a_zero = is_zero(a);
    let b_zero = is_zero(b);
    if a_zero && b_zero {
        // +0 + +0 = +0, -0 + -0 = -0, +0 + -0 = +0
        let result_sign = a.sign & b.sign;
        return make_zero(result_sign, fmt);
    }
    if a_zero {
        return b.clone();
    }
    if b_zero {
        return a.clone();
    }

    // ===================================================================
    // Step 1: Extract exponents and mantissas as integers
    // ===================================================================
    //
    // We work with extended mantissas that include the implicit leading bit.
    // For normal numbers, this is 1; for denormals, it's 0.
    //
    // We also add extra guard bits for rounding precision. The guard bits
    // are: Guard (G), Round (R), and Sticky (S) — 3 extra bits that capture
    // information about bits that would otherwise be lost during shifting.

    let mut exp_a = bits_msb_to_int(&a.exponent) as i32;
    let mut exp_b = bits_msb_to_int(&b.exponent) as i32;
    let mut mant_a = bits_msb_to_int(&a.mantissa) as u64;
    let mut mant_b = bits_msb_to_int(&b.mantissa) as u64;

    // Add implicit leading 1 for normal numbers (exponent != 0)
    if exp_a != 0 {
        mant_a = (1u64 << fmt.mantissa_bits) | mant_a;
    } else {
        exp_a = 1; // Denormal true exponent = 1 - bias, stored as 1 for alignment
    }
    if exp_b != 0 {
        mant_b = (1u64 << fmt.mantissa_bits) | mant_b;
    } else {
        exp_b = 1;
    }

    // Add 3 guard bits (shift left by 3) for rounding precision
    let guard_bits: u32 = 3;
    mant_a <<= guard_bits;
    mant_b <<= guard_bits;

    // ===================================================================
    // Step 2: Align mantissas by shifting the smaller one right
    // ===================================================================

    let result_exp: i32;
    if exp_a >= exp_b {
        let exp_diff = (exp_a - exp_b) as u32;
        if exp_diff > 0 && exp_diff < (fmt.mantissa_bits + 1 + guard_bits) {
            let shifted_out = mant_b & ((1u64 << exp_diff) - 1);
            let sticky = if shifted_out != 0 { 1u64 } else { 0 };
            mant_b >>= exp_diff;
            if sticky != 0 {
                mant_b |= 1;
            }
        } else if exp_diff > 0 {
            let sticky = if mant_b != 0 { 1u64 } else { 0 };
            mant_b >>= exp_diff;
            if sticky != 0 {
                mant_b |= 1;
            }
        }
        result_exp = exp_a;
    } else {
        let exp_diff = (exp_b - exp_a) as u32;
        if exp_diff > 0 && exp_diff < (fmt.mantissa_bits + 1 + guard_bits) {
            let shifted_out = mant_a & ((1u64 << exp_diff) - 1);
            let sticky = if shifted_out != 0 { 1u64 } else { 0 };
            mant_a >>= exp_diff;
            if sticky != 0 {
                mant_a |= 1;
            }
        } else if exp_diff > 0 {
            let sticky = if mant_a != 0 { 1u64 } else { 0 };
            mant_a >>= exp_diff;
            if sticky != 0 {
                mant_a |= 1;
            }
        }
        result_exp = exp_b;
    };
    let mut result_exp = result_exp;

    // ===================================================================
    // Step 3: Add or subtract mantissas based on signs
    // ===================================================================

    let result_mant: u64;
    let result_sign: u8;
    if a.sign == b.sign {
        result_mant = mant_a + mant_b;
        result_sign = a.sign;
    } else if mant_a >= mant_b {
        result_mant = mant_a - mant_b;
        result_sign = a.sign;
    } else {
        result_mant = mant_b - mant_a;
        result_sign = b.sign;
    }
    let mut result_mant = result_mant;

    // ===================================================================
    // Step 4: Handle zero result
    // ===================================================================
    if result_mant == 0 {
        return make_zero(0, fmt); // +0 by convention
    }

    // ===================================================================
    // Step 5: Normalize the result
    // ===================================================================
    //
    // The result mantissa should be in the form 1.xxxx (the leading 1 in
    // position mantissa_bits + guard_bits).

    let normal_pos = fmt.mantissa_bits + guard_bits;
    let leading_pos = bit_length(result_mant) - 1;

    if leading_pos > normal_pos {
        // Overflow: shift right to normalize
        let shift_amount = leading_pos - normal_pos;
        let lost_bits = result_mant & ((1u64 << shift_amount) - 1);
        result_mant >>= shift_amount;
        if lost_bits != 0 {
            result_mant |= 1; // sticky
        }
        result_exp += shift_amount as i32;
    } else if leading_pos < normal_pos {
        // Underflow: shift left to normalize
        let shift_amount = normal_pos - leading_pos;
        if result_exp - shift_amount as i32 >= 1 {
            result_mant <<= shift_amount;
            result_exp -= shift_amount as i32;
        } else {
            // Can't shift all the way — result becomes denormal
            let actual_shift = result_exp - 1;
            if actual_shift > 0 {
                result_mant <<= actual_shift as u32;
            }
            result_exp = 0;
        }
    }

    // ===================================================================
    // Step 6: Round to nearest even
    // ===================================================================
    //
    // Round to nearest even rules:
    //   - If GRS = 0xx: round down (truncate)
    //   - If GRS = 100: round to even (round up if mantissa LSB is 1)
    //   - If GRS = 101, 110, 111: round up

    let guard = (result_mant >> (guard_bits - 1)) & 1;
    let round_bit = (result_mant >> (guard_bits - 2)) & 1;
    let mut sticky_bit = result_mant & ((1u64 << (guard_bits - 2)) - 1);
    if sticky_bit != 0 {
        sticky_bit = 1;
    }

    // Remove guard bits
    result_mant >>= guard_bits;

    // Apply rounding
    if guard == 1 {
        if round_bit == 1 || sticky_bit == 1 {
            result_mant += 1; // Round up
        } else if (result_mant & 1) == 1 {
            result_mant += 1; // Tie-breaking: round to even
        }
    }

    // Check if rounding caused overflow
    if result_mant >= (1u64 << (fmt.mantissa_bits + 1)) {
        result_mant >>= 1;
        result_exp += 1;
    }

    // ===================================================================
    // Step 7: Handle exponent overflow/underflow
    // ===================================================================
    let max_exp = (1i32 << fmt.exponent_bits) - 1;

    if result_exp >= max_exp {
        return make_inf(result_sign, fmt);
    }

    if result_exp <= 0 {
        if result_exp < -(fmt.mantissa_bits as i32) {
            return make_zero(result_sign, fmt);
        }
        let shift = (1 - result_exp) as u32;
        result_mant >>= shift;
        result_exp = 0;
    }

    // ===================================================================
    // Step 8: Pack the result
    // ===================================================================
    // Remove the implicit leading 1 (if normal)
    if result_exp > 0 {
        result_mant &= (1u64 << fmt.mantissa_bits) - 1;
    }

    FloatBits {
        sign: result_sign,
        exponent: int_to_bits_msb(result_exp as u64, fmt.exponent_bits),
        mantissa: int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt,
    }
}

/// Subtracts two floating-point numbers: `a - b`.
///
/// # Why subtraction is trivial once you have addition
///
/// In IEEE 754, `a - b = a + (-b)`. To negate `b`, we just flip its sign bit.
/// This is a single XOR gate in hardware — the cheapest possible operation.
pub fn fp_sub(a: &FloatBits, b: &FloatBits) -> FloatBits {
    let neg_b = FloatBits {
        sign: b.sign ^ 1,
        exponent: b.exponent.clone(),
        mantissa: b.mantissa.clone(),
        fmt: b.fmt,
    };
    fp_add(a, &neg_b)
}

/// Negates a floating-point number: returns `-a`.
///
/// This is the simplest floating-point operation: just flip the sign bit.
/// In hardware, it's literally one NOT gate (or XOR with 1).
///
/// Note: `neg(+0) = -0` and `neg(-0) = +0`. Both are valid IEEE 754 zeros.
pub fn fp_neg(a: &FloatBits) -> FloatBits {
    FloatBits {
        sign: a.sign ^ 1,
        exponent: a.exponent.clone(),
        mantissa: a.mantissa.clone(),
        fmt: a.fmt,
    }
}

/// Returns the absolute value of a floating-point number.
///
/// Even simpler than negation: just force the sign bit to 0.
/// In hardware, this is done by AND-ing the sign bit with 0.
///
/// Note: `abs(NaN)` is still NaN (with sign=0). This is the IEEE 754 behavior.
pub fn fp_abs(a: &FloatBits) -> FloatBits {
    FloatBits {
        sign: 0,
        exponent: a.exponent.clone(),
        mantissa: a.mantissa.clone(),
        fmt: a.fmt,
    }
}

/// Compares two floating-point numbers.
///
/// Returns:
///   - `-1` if `a < b`
///   - `0` if `a == b`
///   - `1` if `a > b`
///
/// NaN comparisons always return 0 (unordered).
///
/// # How FP comparison works in hardware
///
/// For two positive normal numbers:
///   - Compare exponents first (larger exponent = larger number)
///   - If exponents equal, compare mantissas
///
/// For mixed signs: positive > negative (always).
/// For two negative numbers: comparison is reversed.
pub fn fp_compare(a: &FloatBits, b: &FloatBits) -> i32 {
    // NaN is unordered
    if is_nan(a) || is_nan(b) {
        return 0;
    }

    // Handle zeros: +0 == -0
    if is_zero(a) && is_zero(b) {
        return 0;
    }

    // Different signs: positive > negative
    if a.sign != b.sign {
        if is_zero(a) {
            return if b.sign == 1 { 1 } else { -1 };
        }
        if is_zero(b) {
            return if a.sign == 1 { -1 } else { 1 };
        }
        return if a.sign == 1 { -1 } else { 1 };
    }

    // Same sign: compare exponent, then mantissa
    let exp_a = bits_msb_to_int(&a.exponent);
    let exp_b = bits_msb_to_int(&b.exponent);
    let mant_a = bits_msb_to_int(&a.mantissa);
    let mant_b = bits_msb_to_int(&b.mantissa);

    if exp_a != exp_b {
        return if a.sign == 0 {
            if exp_a > exp_b { 1 } else { -1 }
        } else {
            if exp_a > exp_b { -1 } else { 1 }
        };
    }

    if mant_a != mant_b {
        return if a.sign == 0 {
            if mant_a > mant_b { 1 } else { -1 }
        } else {
            if mant_a > mant_b { -1 } else { 1 }
        };
    }

    0
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ieee754::{float_to_bits, bits_to_float};
    use crate::formats::FP32;

    #[test]
    fn test_fp_add_basic() {
        let cases: Vec<(&str, f64, f64, f64)> = vec![
            ("1.5 + 2.5 = 4.0", 1.5, 2.5, 4.0),
            ("1.0 + 1.0 = 2.0", 1.0, 1.0, 2.0),
            ("0.5 + 0.25 = 0.75", 0.5, 0.25, 0.75),
            ("100.0 + 0.01", 100.0, 0.01, (100.0_f32 + 0.01_f32) as f64),
            ("1.5 + 0.0 = 1.5", 1.5, 0.0, 1.5),
            ("0.0 + 2.5 = 2.5", 0.0, 2.5, 2.5),
        ];

        for (name, av, bv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let result = fp_add(&a, &b);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FPAdd {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fp_add_negative() {
        let cases: Vec<(&str, f64, f64, f64)> = vec![
            ("-1.0 + -2.0 = -3.0", -1.0, -2.0, -3.0),
            ("1.0 + -0.5 = 0.5", 1.0, -0.5, 0.5),
            ("-1.0 + 2.0 = 1.0", -1.0, 2.0, 1.0),
            ("1.0 + -1.0 = 0.0", 1.0, -1.0, 0.0),
        ];

        for (name, av, bv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let result = fp_add(&a, &b);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FPAdd {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fp_add_special_values() {
        let nan = float_to_bits(f64::NAN, FP32);
        let inf = float_to_bits(f64::INFINITY, FP32);
        let neg_inf = float_to_bits(f64::NEG_INFINITY, FP32);
        let one = float_to_bits(1.0, FP32);
        let zero = float_to_bits(0.0, FP32);

        // NaN + anything = NaN
        assert!(is_nan(&fp_add(&nan, &one)));
        assert!(is_nan(&fp_add(&one, &nan)));

        // Inf + (-Inf) = NaN
        assert!(is_nan(&fp_add(&inf, &neg_inf)));

        // Inf + Inf = Inf
        let r = fp_add(&inf, &inf);
        assert!(is_inf(&r) && r.sign == 0);

        // Inf + finite = Inf
        let r = fp_add(&inf, &one);
        assert!(is_inf(&r) && r.sign == 0);

        // 0 + 0 = 0
        assert!(is_zero(&fp_add(&zero, &zero)));
    }

    #[test]
    fn test_fp_sub() {
        let cases: Vec<(&str, f64, f64, f64)> = vec![
            ("3.0 - 1.0 = 2.0", 3.0, 1.0, 2.0),
            ("1.0 - 3.0 = -2.0", 1.0, 3.0, -2.0),
            ("5.0 - 5.0 = 0.0", 5.0, 5.0, 0.0),
            ("-1.0 - -1.0 = 0.0", -1.0, -1.0, 0.0),
        ];

        for (name, av, bv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let result = fp_sub(&a, &b);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FPSub {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fp_neg() {
        // Negate positive
        let pos = float_to_bits(3.14, FP32);
        let neg = fp_neg(&pos);
        assert_eq!(neg.sign, 1);
        assert_eq!(bits_to_float(&neg) as f32, -3.14_f32);

        // Double negation returns original
        let double_neg = fp_neg(&neg);
        assert_eq!(double_neg.sign, 0);

        // Negate zero
        let pos_zero = float_to_bits(0.0, FP32);
        let neg_zero = fp_neg(&pos_zero);
        assert_eq!(neg_zero.sign, 1);
    }

    #[test]
    fn test_fp_abs() {
        let neg = float_to_bits(-5.0, FP32);
        let abs_val = fp_abs(&neg);
        assert_eq!(abs_val.sign, 0);
        assert_eq!(bits_to_float(&abs_val) as f32, 5.0);

        // abs of positive stays positive
        let pos = float_to_bits(5.0, FP32);
        let abs_pos = fp_abs(&pos);
        assert_eq!(abs_pos.sign, 0);
    }

    #[test]
    fn test_fp_compare() {
        let cases: Vec<(&str, f64, f64, i32)> = vec![
            ("1.0 < 2.0", 1.0, 2.0, -1),
            ("2.0 > 1.0", 2.0, 1.0, 1),
            ("1.0 == 1.0", 1.0, 1.0, 0),
            ("-1.0 < 1.0", -1.0, 1.0, -1),
            ("1.0 > -1.0", 1.0, -1.0, 1),
            ("-2.0 < -1.0", -2.0, -1.0, -1),
            ("-1.0 > -2.0", -1.0, -2.0, 1),
            ("+0 == -0", 0.0, -0.0, 0),
        ];

        for (name, av, bv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let got = fp_compare(&a, &b);
            assert_eq!(got, want, "FPCompare {name}: got {got}, want {want}");
        }

        // NaN comparisons return 0 (unordered)
        let nan = float_to_bits(f64::NAN, FP32);
        let one = float_to_bits(1.0, FP32);
        assert_eq!(fp_compare(&nan, &one), 0);
        assert_eq!(fp_compare(&one, &nan), 0);
    }

    #[test]
    fn test_fp_add_commutative() {
        let pairs: Vec<(f64, f64)> = vec![(1.5, 2.5), (-3.0, 7.0), (0.001, 1000.0)];

        for (av, bv) in pairs {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let ab = bits_to_float(&fp_add(&a, &b));
            let ba = bits_to_float(&fp_add(&b, &a));
            assert_eq!(
                ab as f32, ba as f32,
                "FPAdd not commutative: {av} + {bv} = {ab}, but {bv} + {av} = {ba}"
            );
        }
    }

    #[test]
    fn test_fp_compare_zero_identity() {
        let pos_zero = float_to_bits(0.0, FP32);
        let one = float_to_bits(1.0, FP32);
        let neg_one = float_to_bits(-1.0, FP32);

        assert_eq!(fp_compare(&pos_zero, &one), -1);
        assert_eq!(fp_compare(&pos_zero, &neg_one), 1);
    }
}
