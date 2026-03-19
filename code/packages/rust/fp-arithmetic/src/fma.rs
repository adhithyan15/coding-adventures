//! Fused Multiply-Add and format conversion.
//!
//! # What is FMA (Fused Multiply-Add)?
//!
//! FMA computes `a * b + c` with only ONE rounding step at the end. Compare:
//!
//! ```text
//! Without FMA (separate operations):
//!     temp = fp_mul(a, b)       // round #1 (loses precision)
//!     result = fp_add(temp, c)  // round #2 (loses more precision)
//!
//! With FMA:
//!     result = fp_fma(a, b, c)  // round only once!
//! ```
//!
//! # Why FMA matters for ML
//!
//! In machine learning, the dominant computation is the dot product:
//!
//! ```text
//! result = sum(a[i] * w[i] for i in 0..N)
//! ```
//!
//! Each multiply-add in the sum is a potential FMA. By rounding only once per
//! operation instead of twice, FMA gives more accurate gradients during training.
//!
//! Every modern processor has FMA:
//!   - Intel Haswell (2013): FMA3 instruction (AVX2)
//!   - NVIDIA GPUs: native FMA in CUDA cores
//!   - Google TPU: the MAC (Multiply-Accumulate) unit IS an FMA
//!   - Apple M-series: FMA in both CPU and Neural Engine
//!
//! # Algorithm
//!
//! ```text
//! Step 1: Multiply a * b with FULL precision (no rounding!)
//! Step 2: Align c's mantissa to the product's exponent
//! Step 3: Add the full-precision product and aligned c
//! Step 4: Normalize and round ONCE
//! ```

use crate::formats::{FloatFormat, FloatBits, make_nan, make_inf, make_zero};
use crate::ieee754::{
    bits_msb_to_int, int_to_bits_msb, is_nan, is_inf, is_zero, bit_length,
    float_to_bits, bits_to_float,
};

/// Computes `a * b + c` with a single rounding step (fused multiply-add).
///
/// # Worked example: FMA(1.5, 2.0, 0.25) in FP32
///
/// ```text
/// a = 1.5 = 1.1 x 2^0    (exp=127, mant=1.100...0)
/// b = 2.0 = 1.0 x 2^1    (exp=128, mant=1.000...0)
/// c = 0.25 = 1.0 x 2^-2  (exp=125, mant=1.000...0)
///
/// Step 1: Full-precision multiply
///         1.100...0 x 1.000...0 = 1.100...0 (48-bit, no rounding)
///         Product exponent: 127 + 128 - 127 = 128 (true exp = 1)
///         So product = 1.1 x 2^1 = 3.0
///
/// Step 2: Align c to product's exponent
///         c = 1.0 x 2^-2, product exponent = 128
///         Shift c right by 128 - 125 = 3 positions
///
/// Step 3: Add
///         1.100 x 2^1 + 0.001 x 2^1 = 1.101 x 2^1
///
/// Step 4: Normalize and round
///         Already normalized, result = 1.101 x 2^1 = 3.25
///         Check: 1.5 * 2.0 + 0.25 = 3.0 + 0.25 = 3.25 correct!
/// ```
pub fn fp_fma(a: &FloatBits, b: &FloatBits, c: &FloatBits) -> FloatBits {
    let fmt = a.fmt;

    // ===================================================================
    // Step 0: Handle special cases
    // ===================================================================
    if is_nan(a) || is_nan(b) || is_nan(c) {
        return make_nan(fmt);
    }

    let a_inf = is_inf(a);
    let b_inf = is_inf(b);
    let c_inf = is_inf(c);
    let a_zero = is_zero(a);
    let b_zero = is_zero(b);

    // Inf * 0 = NaN
    if (a_inf && b_zero) || (b_inf && a_zero) {
        return make_nan(fmt);
    }

    let product_sign = a.sign ^ b.sign;

    // Inf * finite + c
    if a_inf || b_inf {
        if c_inf && product_sign != c.sign {
            return make_nan(fmt); // Inf + (-Inf) = NaN
        }
        return make_inf(product_sign, fmt);
    }

    // a * b = 0, result is just c
    if a_zero || b_zero {
        if is_zero(c) {
            let result_sign = product_sign & c.sign;
            return make_zero(result_sign, fmt);
        }
        return c.clone();
    }

    // c is Inf
    if c_inf {
        return c.clone();
    }

    // ===================================================================
    // Step 1: Multiply a * b with full precision (no rounding!)
    // ===================================================================
    let mut exp_a = bits_msb_to_int(&a.exponent) as i32;
    let mut exp_b = bits_msb_to_int(&b.exponent) as i32;
    let mut mant_a = bits_msb_to_int(&a.mantissa) as u64;
    let mut mant_b = bits_msb_to_int(&b.mantissa) as u64;

    if exp_a != 0 {
        mant_a = (1u64 << fmt.mantissa_bits) | mant_a;
    } else {
        exp_a = 1;
    }
    if exp_b != 0 {
        mant_b = (1u64 << fmt.mantissa_bits) | mant_b;
    } else {
        exp_b = 1;
    }

    // Full-precision product: no truncation, no rounding!
    let mut product = mant_a * mant_b;
    let mut product_exp = exp_a + exp_b - fmt.bias;

    // Normalize the product
    let product_leading = bit_length(product) - 1;
    let normal_product_pos = 2 * fmt.mantissa_bits;

    if product_leading > normal_product_pos {
        product_exp += (product_leading - normal_product_pos) as i32;
    } else if product_leading < normal_product_pos {
        product_exp -= (normal_product_pos - product_leading) as i32;
    }

    // ===================================================================
    // Step 2: Align c's mantissa to the product's exponent
    // ===================================================================
    let mut exp_c = bits_msb_to_int(&c.exponent) as i32;
    let mut mant_c = bits_msb_to_int(&c.mantissa) as u64;

    if exp_c != 0 {
        mant_c = (1u64 << fmt.mantissa_bits) | mant_c;
    } else {
        exp_c = 1;
    }

    let exp_diff = product_exp - exp_c;

    let c_scale_shift = product_leading as i32 - fmt.mantissa_bits as i32;
    let mut c_aligned: u64;
    if c_scale_shift >= 0 {
        c_aligned = mant_c << (c_scale_shift as u32);
    } else {
        c_aligned = mant_c >> ((-c_scale_shift) as u32);
    }

    let result_exp: i32;
    if exp_diff >= 0 {
        // Clamp shift to avoid overflow — if exp_diff >= 64, the value
        // is shifted entirely to zero (it's too small to contribute).
        let shift = (exp_diff as u32).min(63);
        c_aligned >>= shift;
        result_exp = product_exp;
    } else {
        let shift = ((-exp_diff) as u32).min(63);
        product >>= shift;
        result_exp = exp_c;
    }
    let mut result_exp = result_exp;

    // ===================================================================
    // Step 3: Add product and c
    // ===================================================================
    let mut result_mant: u64;
    let result_sign: u8;
    if product_sign == c.sign {
        result_mant = product + c_aligned;
        result_sign = product_sign;
    } else if product >= c_aligned {
        result_mant = product - c_aligned;
        result_sign = product_sign;
    } else {
        result_mant = c_aligned - product;
        result_sign = c.sign;
    }

    if result_mant == 0 {
        return make_zero(0, fmt);
    }

    // ===================================================================
    // Step 4: Normalize and round ONCE
    // ===================================================================
    let result_leading = bit_length(result_mant) - 1;
    let mut target_pos = product_leading;
    if target_pos < fmt.mantissa_bits {
        target_pos = fmt.mantissa_bits;
    }

    if result_leading > target_pos {
        let shift = result_leading - target_pos;
        result_exp += shift as i32;
    } else if result_leading < target_pos {
        let shift_needed = target_pos - result_leading;
        result_exp -= shift_needed as i32;
    }

    // Round to mantissa_bits precision
    let result_leading = bit_length(result_mant) - 1;
    let round_pos = result_leading as i32 - fmt.mantissa_bits as i32;

    if round_pos > 0 {
        let rp = round_pos as u32;
        let guard = (result_mant >> (rp - 1)) & 1;
        let mut round_bit = 0u64;
        let mut sticky = 0u64;
        if rp >= 2 {
            round_bit = (result_mant >> (rp - 2)) & 1;
            let mask = (1u64 << (rp - 2)) - 1;
            if result_mant & mask != 0 {
                sticky = 1;
            }
        }

        result_mant >>= rp;

        // Round to nearest even
        if guard == 1 {
            if round_bit == 1 || sticky == 1 {
                result_mant += 1;
            } else if (result_mant & 1) == 1 {
                result_mant += 1;
            }
        }

        if result_mant >= (1u64 << (fmt.mantissa_bits + 1)) {
            result_mant >>= 1;
            result_exp += 1;
        }
    } else if round_pos < 0 {
        result_mant <<= (-round_pos) as u32;
    }

    // Handle exponent overflow/underflow
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

    // Remove implicit leading 1
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

/// Converts a floating-point number from one format to another.
///
/// # Why format conversion matters
///
/// In ML pipelines, data frequently changes precision:
///   - Training starts in FP32 (full precision)
///   - Forward pass uses FP16 or BF16 (faster, less memory)
///   - Gradients accumulated in FP32 (need precision)
///   - Weights stored as BF16 on TPU
///
/// # FP32 -> BF16 conversion (trivially simple!)
///
/// BF16 was designed so that conversion from FP32 is dead simple:
/// just truncate the lower 16 bits! Both formats use the same 8-bit
/// exponent with bias 127, so no exponent adjustment is needed.
///
/// ```text
/// FP32: [sign(1)] [exponent(8)] [mantissa(23)]
/// BF16: [sign(1)] [exponent(8)] [mantissa(7) ]
///                                ^^^^^^^^^^^ just take the top 7 of 23
/// ```
pub fn fp_convert(bits: &FloatBits, target_fmt: FloatFormat) -> FloatBits {
    // Same format: no conversion needed
    if bits.fmt == target_fmt {
        return bits.clone();
    }

    // Strategy: decode to Rust f64, then re-encode in target format.
    // This handles all edge cases correctly.
    let value = bits_to_float(bits);
    float_to_bits(value, target_fmt)
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{FP32, FP16, BF16};
    use crate::fp_multiplier::fp_mul;

    #[test]
    fn test_fma_basic() {
        let cases: Vec<(&str, f64, f64, f64, f64)> = vec![
            ("1.5 * 2.0 + 0.25 = 3.25", 1.5, 2.0, 0.25, 3.25),
            ("2.0 * 3.0 + 1.0 = 7.0", 2.0, 3.0, 1.0, 7.0),
            ("1.0 * 1.0 + 0.0 = 1.0", 1.0, 1.0, 0.0, 1.0),
            ("0.5 * 0.5 + 0.5 = 0.75", 0.5, 0.5, 0.5, 0.75),
            ("10.0 * 10.0 + 0.0 = 100.0", 10.0, 10.0, 0.0, 100.0),
        ];

        for (name, av, bv, cv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let c = float_to_bits(cv, FP32);
            let result = fp_fma(&a, &b, &c);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FMA {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fma_with_negatives() {
        let cases: Vec<(&str, f64, f64, f64, f64)> = vec![
            ("-1.0 * 2.0 + 3.0 = 1.0", -1.0, 2.0, 3.0, 1.0),
            ("2.0 * -3.0 + 10.0 = 4.0", 2.0, -3.0, 10.0, 4.0),
            ("-2.0 * -3.0 + 1.0 = 7.0", -2.0, -3.0, 1.0, 7.0),
        ];

        for (name, av, bv, cv, want) in cases {
            let a = float_to_bits(av, FP32);
            let b = float_to_bits(bv, FP32);
            let c = float_to_bits(cv, FP32);
            let result = fp_fma(&a, &b, &c);
            let got = bits_to_float(&result);
            assert_eq!(got as f32, want as f32, "FMA {name}: got {got}, want {want}");
        }
    }

    #[test]
    fn test_fma_special_values() {
        let nan = float_to_bits(f64::NAN, FP32);
        let inf = float_to_bits(f64::INFINITY, FP32);
        let neg_inf = float_to_bits(f64::NEG_INFINITY, FP32);
        let zero = float_to_bits(0.0, FP32);
        let one = float_to_bits(1.0, FP32);
        let two = float_to_bits(2.0, FP32);

        // NaN in any position -> NaN
        assert!(is_nan(&fp_fma(&nan, &one, &one)));
        assert!(is_nan(&fp_fma(&one, &nan, &one)));
        assert!(is_nan(&fp_fma(&one, &one, &nan)));

        // Inf * 0 = NaN
        assert!(is_nan(&fp_fma(&inf, &zero, &one)));
        assert!(is_nan(&fp_fma(&zero, &inf, &one)));

        // Inf * finite + c = Inf
        assert!(is_inf(&fp_fma(&inf, &two, &one)));

        // Inf * finite + (-Inf) = NaN
        assert!(is_nan(&fp_fma(&inf, &one, &neg_inf)));

        // 0 * 0 + 0 = 0
        assert!(is_zero(&fp_fma(&zero, &zero, &zero)));

        // 0 * finite + c = c
        let five = float_to_bits(5.0, FP32);
        let three = float_to_bits(3.0, FP32);
        let r = fp_fma(&zero, &five, &three);
        assert_eq!(bits_to_float(&r) as f32, 3.0);

        // finite * finite + Inf = Inf
        assert!(is_inf(&fp_fma(&two, &three, &inf)));
    }

    #[test]
    fn test_fp_convert() {
        // Same format: no-op
        let bits = float_to_bits(3.14, FP32);
        let converted = fp_convert(&bits, FP32);
        assert_eq!(bits_to_float(&converted), bits_to_float(&bits));

        // FP32 -> FP16 -> FP32 round-trip
        let bits32 = float_to_bits(1.5, FP32);
        let bits16 = fp_convert(&bits32, FP16);
        let bits_back = fp_convert(&bits16, FP32);
        assert_eq!(bits_to_float(&bits_back) as f32, 1.5);

        // FP32 -> BF16 -> FP32 round-trip
        let bits32 = float_to_bits(1.0, FP32);
        let bits_bf = fp_convert(&bits32, BF16);
        let bits_back = fp_convert(&bits_bf, FP32);
        assert_eq!(bits_to_float(&bits_back) as f32, 1.0);

        // Special values survive conversion
        assert!(is_nan(&fp_convert(&float_to_bits(f64::NAN, FP32), FP16)));
        assert!(is_inf(&fp_convert(&float_to_bits(f64::INFINITY, FP32), FP16)));
        assert!(is_zero(&fp_convert(&float_to_bits(0.0, FP32), FP16)));
    }

    /// Simulates a small dot product using FMA.
    ///
    /// dot(a, w) = a[0]*w[0] + a[1]*w[1] + a[2]*w[2]
    /// = FMA(a[2], w[2], FMA(a[1], w[1], a[0]*w[0]))
    #[test]
    fn test_fma_dot_product() {
        // [1, 2, 3] . [4, 5, 6] = 4 + 10 + 18 = 32
        let a_vals = [1.0, 2.0, 3.0];
        let w_vals = [4.0, 5.0, 6.0];

        // Start with first multiply
        let mut acc = fp_mul(
            &float_to_bits(a_vals[0], FP32),
            &float_to_bits(w_vals[0], FP32),
        );

        // Accumulate using FMA
        for i in 1..a_vals.len() {
            acc = fp_fma(
                &float_to_bits(a_vals[i], FP32),
                &float_to_bits(w_vals[i], FP32),
                &acc,
            );
        }

        assert_eq!(bits_to_float(&acc) as f32, 32.0);
    }
}
