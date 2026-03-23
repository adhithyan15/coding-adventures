//! Floating-point multiplication — built from first principles.
//!
//! # How FP multiplication works
//!
//! Floating-point multiplication is actually simpler than addition! That's because
//! you don't need to align mantissas — the exponents just add together.
//!
//! In scientific notation:
//!
//! ```text
//! (1.5 x 10^3) x (2.0 x 10^2) = (1.5 x 2.0) x 10^(3+2) = 3.0 x 10^5
//! ```
//!
//! The same principle applies in binary:
//!
//! ```text
//! (-1)^s1 x 1.m1 x 2^e1  *  (-1)^s2 x 1.m2 x 2^e2
//! = (-1)^(s1 XOR s2) x (1.m1 x 1.m2) x 2^(e1 + e2)
//! ```
//!
//! # The four steps of FP multiplication
//!
//! ```text
//! Step 1: Result sign = XOR of input signs
//!         Positive x Positive = Positive (0 XOR 0 = 0)
//!         Positive x Negative = Negative (0 XOR 1 = 1)
//!         Negative x Negative = Positive (1 XOR 1 = 0)
//!
//! Step 2: Result exponent = exp_a + exp_b - bias
//!         We subtract the bias once because both exponents include it.
//!
//! Step 3: Multiply mantissas using shift-and-add
//!         The result is double-width (e.g., 48 bits for FP32's 24-bit mantissas).
//!
//! Step 4: Normalize and round (same as addition)
//! ```
//!
//! # Shift-and-add multiplication
//!
//! Binary multiplication works like long multiplication but simpler because
//! each digit is only 0 or 1:
//!
//! ```text
//!   1.101  (multiplicand = 1.625 in decimal)
//! x 1.011  (multiplier   = 1.375 in decimal)
//! -------
//!   1101   (1.101 x 1)     -- multiplier bit 0 is 1, add
//!  1101    (1.101 x 1)     -- multiplier bit 1 is 1, add (shifted left 1)
//! 0000     (1.101 x 0)     -- multiplier bit 2 is 0, skip
//! 1101      (1.101 x 1)   -- multiplier bit 3 is 1, add (shifted left 3)
//! ---------
//! 10.001111  = 2.234375 in decimal
//! ```
//!
//! Check: 1.625 x 1.375 = 2.234375 correct!

use crate::formats::{FloatBits, make_nan, make_inf, make_zero};
use crate::ieee754::{bits_msb_to_int, int_to_bits_msb, is_nan, is_inf, is_zero, bit_length};

/// Multiplies two floating-point numbers using the IEEE 754 algorithm.
///
/// # Worked example: 1.5 x 2.0 in FP32
///
/// ```text
/// 1.5 = 1.1 x 2^0    -> sign=0, exp=127, mant=100...0
/// 2.0 = 1.0 x 2^1    -> sign=0, exp=128, mant=000...0
///
/// Step 1: result_sign = 0 XOR 0 = 0 (positive)
/// Step 2: result_exp = 127 + 128 - 127 = 128 (true exp = 1)
/// Step 3: mantissa product:
///         1.100...0 x 1.000...0 = 1.100...0 (trivial case)
/// Step 4: Already normalized
/// Result: 1.1 x 2^1 = 3.0 (correct!)
/// ```
pub fn fp_mul(a: &FloatBits, b: &FloatBits) -> FloatBits {
    let fmt = a.fmt;

    // ===================================================================
    // Step 0: Handle special cases
    // ===================================================================
    // IEEE 754 rules for multiplication:
    //   NaN x anything = NaN
    //   Inf x 0 = NaN
    //   Inf x finite = Inf (with appropriate sign)
    //   0 x finite = 0

    // Result sign: always XOR of input signs (even for special cases)
    let result_sign = a.sign ^ b.sign;

    // NaN propagation
    if is_nan(a) || is_nan(b) {
        return make_nan(fmt);
    }

    let a_inf = is_inf(a);
    let b_inf = is_inf(b);
    let a_zero = is_zero(a);
    let b_zero = is_zero(b);

    // Inf x 0 = NaN (undefined)
    if (a_inf && b_zero) || (b_inf && a_zero) {
        return make_nan(fmt);
    }

    // Inf x anything = Inf
    if a_inf || b_inf {
        return make_inf(result_sign, fmt);
    }

    // Zero x anything = Zero
    if a_zero || b_zero {
        return make_zero(result_sign, fmt);
    }

    // ===================================================================
    // Step 1: Extract exponents and mantissas
    // ===================================================================
    let mut exp_a = bits_msb_to_int(&a.exponent) as i32;
    let mut exp_b = bits_msb_to_int(&b.exponent) as i32;
    let mut mant_a = bits_msb_to_int(&a.mantissa) as u64;
    let mut mant_b = bits_msb_to_int(&b.mantissa) as u64;

    // Add implicit leading 1 for normal numbers
    if exp_a != 0 {
        mant_a = (1u64 << fmt.mantissa_bits) | mant_a;
    } else {
        exp_a = 1; // Denormal: true exponent = 1 - bias
    }

    if exp_b != 0 {
        mant_b = (1u64 << fmt.mantissa_bits) | mant_b;
    } else {
        exp_b = 1;
    }

    // ===================================================================
    // Step 2: Add exponents, subtract bias
    // ===================================================================
    let mut result_exp = exp_a + exp_b - fmt.bias;

    // ===================================================================
    // Step 3: Multiply mantissas (shift-and-add)
    // ===================================================================
    //
    // The mantissa product of two (mantissa_bits+1)-bit numbers produces
    // a (2*(mantissa_bits+1))-bit result. Rust's u64 multiplication handles
    // this for FP32 (24*24 = 48 bits, fits in u64).
    let product = mant_a * mant_b;

    // ===================================================================
    // Step 4: Normalize
    // ===================================================================
    let leading_pos = bit_length(product) - 1;
    let normal_pos = 2 * fmt.mantissa_bits;

    if leading_pos > normal_pos {
        let extra = leading_pos - normal_pos;
        result_exp += extra as i32;
    } else if leading_pos < normal_pos {
        let deficit = normal_pos - leading_pos;
        result_exp -= deficit as i32;
    }

    // ===================================================================
    // Step 5: Round to nearest even
    // ===================================================================
    let round_pos = leading_pos as i32 - fmt.mantissa_bits as i32;

    let mut result_mant: u64;
    if round_pos > 0 {
        let rp = round_pos as u32;
        let guard = (product >> (rp - 1)) & 1;
        let mut round_bit = 0u64;
        let mut sticky = 0u64;
        if rp >= 2 {
            round_bit = (product >> (rp - 2)) & 1;
            let mask = (1u64 << (rp - 2)) - 1;
            if product & mask != 0 {
                sticky = 1;
            }
        }

        result_mant = product >> rp;

        // Apply rounding
        if guard == 1 {
            if round_bit == 1 || sticky == 1 {
                result_mant += 1;
            } else if (result_mant & 1) == 1 {
                result_mant += 1;
            }
        }

        // Check if rounding caused mantissa overflow
        if result_mant >= (1u64 << (fmt.mantissa_bits + 1)) {
            result_mant >>= 1;
            result_exp += 1;
        }
    } else if round_pos == 0 {
        result_mant = product;
    } else {
        result_mant = product << ((-round_pos) as u32);
    }

    // ===================================================================
    // Step 6: Handle exponent overflow/underflow
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
    // Step 7: Pack the result
    // ===================================================================
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

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ieee754::{float_to_bits, bits_to_float};
    use crate::formats::FP32;

    #[test]
    fn test_fp_mul_basic() {
        let cases: Vec<(&str, f64, f64, f64)> = vec![
            ("1.5 * 2.0 = 3.0", 1.5, 2.0, 3.0),
            ("3.0 * 4.0 = 12.0", 3.0, 4.0, 12.0),
            ("0.5 * 0.5 = 0.25", 0.5, 0.5, 0.25),
            ("10.0 * 0.1", 10.0, 0.1, (10.0_f32 * 0.1_f32) as f64),
            ("1.0 * 1.0 = 1.0", 1.0, 1.0, 1.0),
        ];

        for (name, av, bv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let result = fp_mul(&a, &b);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FPMul {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fp_mul_signs() {
        let cases: Vec<(&str, f64, f64, f64)> = vec![
            ("pos * pos = pos", 2.0, 3.0, 6.0),
            ("pos * neg = neg", 2.0, -3.0, -6.0),
            ("neg * pos = neg", -2.0, 3.0, -6.0),
            ("neg * neg = pos", -2.0, -3.0, 6.0),
        ];

        for (name, av, bv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let result = fp_mul(&a, &b);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FPMul {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fp_mul_special_values() {
        let nan = float_to_bits(f64::NAN, FP32);
        let inf = float_to_bits(f64::INFINITY, FP32);
        let neg_inf = float_to_bits(f64::NEG_INFINITY, FP32);
        let zero = float_to_bits(0.0, FP32);
        let one = float_to_bits(1.0, FP32);
        let two = float_to_bits(2.0, FP32);

        // NaN * anything = NaN
        assert!(is_nan(&fp_mul(&nan, &one)));

        // Inf * 0 = NaN
        assert!(is_nan(&fp_mul(&inf, &zero)));
        assert!(is_nan(&fp_mul(&zero, &inf)));

        // Inf * finite = Inf
        let r = fp_mul(&inf, &two);
        assert!(is_inf(&r) && r.sign == 0);

        // Inf * -finite = -Inf
        let neg_two = float_to_bits(-2.0, FP32);
        let r = fp_mul(&inf, &neg_two);
        assert!(is_inf(&r) && r.sign == 1);

        // -Inf * -Inf = +Inf
        let r = fp_mul(&neg_inf, &neg_inf);
        assert!(is_inf(&r) && r.sign == 0);

        // 0 * finite = 0
        let five = float_to_bits(5.0, FP32);
        assert!(is_zero(&fp_mul(&zero, &five)));
    }

    #[test]
    fn test_fp_mul_overflow() {
        let huge = float_to_bits(1e38, FP32);
        assert!(is_inf(&fp_mul(&huge, &huge)));
    }

    #[test]
    fn test_fp_mul_commutative() {
        let pairs: Vec<(f64, f64)> = vec![(1.5, 2.5), (-3.0, 7.0), (0.001, 1000.0)];

        for (av, bv) in pairs {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let ab = bits_to_float(&fp_mul(&a, &b));
            let ba = bits_to_float(&fp_mul(&b, &a));
            assert_eq!(ab as f32, ba as f32, "FPMul not commutative for {av} * {bv}");
        }
    }

    #[test]
    fn test_fp_mul_by_one() {
        let values = [1.0, -1.0, 3.14, -0.001, 1e10, 1e-10];
        let one = float_to_bits(1.0, FP32);

        for &v in &values {
            let a = float_to_bits(v, FP32);
            let result = fp_mul(&a, &one);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, v as f32, "FPMul({v}, 1.0) = {got}, want {v}");
        }
    }

    #[test]
    fn test_fp_mul_by_zero() {
        let values = [1.0, -5.0, 1e30];
        let zero = float_to_bits(0.0, FP32);

        for &v in &values {
            let a = float_to_bits(v, FP32);
            assert!(is_zero(&fp_mul(&a, &zero)), "FPMul({v}, 0) should be zero");
        }
    }
}
