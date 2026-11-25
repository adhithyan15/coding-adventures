//! Deterministic, cross-platform IEEE-754 transcendental helpers.
//!
//! Avoids all `std` `libm` calls (`f64::exp`, `f64::ln`, `f64::powf`,
//! `f64::asin`, `f64::atan2`, `f64::sqrt`) so atmospheric pressure, density,
//! and aerodynamic angles produce bit-identical results across Windows, Linux,
//! and macOS.
//!
//! Every arithmetic operation is written as separate statements without
//! `mul_add` to prevent fused multiply-add contraction differences.

/// High 42 bits of `ln(2)` so `k * LN2_HI` is exact for `|k| <= 2048`.
const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fefa_3800);
/// Low tail of `ln(2)` (`LN2_HI + LN2_LO == ln(2)` to ~85 bits).
const LN2_LO: f64 = f64::from_bits(0x3d2e_f357_93c7_6730);
/// Reciprocal `1 / ln(2)` (`0x3ff7_1547_652b_82fe`).
const INV_LN2: f64 = f64::from_bits(0x3ff7_1547_652b_82fe);
/// Rounding magic constant `1.5 * 2^52` for exact round-to-nearest integer.
const ROUND_MAGIC: f64 = f64::from_bits(0x4338_0000_0000_0000);
/// `sqrt(2)` (`0x3ff6_a09e_667f_3bcd`).
const SQRT2: f64 = f64::from_bits(0x3ff6_a09e_667f_3bcd);
/// `2^54` for normalizing subnormal `f64` inputs in `det_ln`.
const TWO_POW_54: f64 = f64::from_bits((1023_u64 + 54_u64) << 52);

const EXP_OVERFLOW_BOUND: f64 = 709.782_712_893_384;
const EXP_UNDERFLOW_BOUND: f64 = -745.133_219_101_941_2;

/// Scale `y` by `2^k` using IEEE-754 exponent bit construction (`-1074 <= k <= 1024`).
#[inline]
fn scale_pow2(y: f64, k: i64) -> f64 {
    if (-1022..=1023).contains(&k) {
        let exp_bits = ((k + 1023) as u64) << 52;
        return y * f64::from_bits(exp_bits);
    }
    if k == 1024 {
        let half_scale = f64::from_bits(2046_u64 << 52);
        return (y * 2.0) * half_scale;
    }
    if k > 1024 {
        return f64::INFINITY;
    }
    if k < -1075 {
        return 0.0;
    }
    // Subnormal range: k in [-1075, -1023]
    let shift = k + 1022; // in [-53, -1]
    let s_sub = f64::from_bits(1_u64 << 52); // 2^-1022
    let s_rem = f64::from_bits(((shift + 1023) as u64) << 52);
    (y * s_rem) * s_sub
}

/// Deterministic `exp(x)` using Cody-Waite range reduction (`x = k*ln(2) + r`,
/// `|r| <= ln(2)/2`) and a degree-14 Horner Taylor polynomial.
///
/// Relative error is `< 1e-15` across the full normal `f64` domain, with zero
/// `std` `libm` calls.
pub fn det_exp(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x > EXP_OVERFLOW_BOUND {
        return f64::INFINITY;
    }
    if x < EXP_UNDERFLOW_BOUND {
        return 0.0;
    }
    if x == 0.0 {
        return 1.0;
    }

    let t = x * INV_LN2;
    let kf = (t + ROUND_MAGIC) - ROUND_MAGIC;
    let k = kf as i64;

    let k_hi = kf * LN2_HI;
    let hi = x - k_hi;
    let lo = kf * LN2_LO;
    let r = hi - lo;

    // Degree-14 Taylor polynomial for (exp(r) - 1 - r) / r^2:
    // sum_{n=2}^{14} r^{n-2} / n!
    let mut p = 1.0 / 87_178_291_200.0; // 1/14!
    p = (1.0 / 6_227_020_800.0) + r * p; // 1/13!
    p = (1.0 / 479_001_600.0) + r * p; // 1/12!
    p = (1.0 / 39_916_800.0) + r * p; // 1/11!
    p = (1.0 / 3_628_800.0) + r * p; // 1/10!
    p = (1.0 / 362_880.0) + r * p; // 1/9!
    p = (1.0 / 40_320.0) + r * p; // 1/8!
    p = (1.0 / 5_040.0) + r * p; // 1/7!
    p = (1.0 / 720.0) + r * p; // 1/6!
    p = (1.0 / 120.0) + r * p; // 1/5!
    p = (1.0 / 24.0) + r * p; // 1/4!
    p = (1.0 / 6.0) + r * p; // 1/3!
    p = 0.5 + r * p; // 1/2!

    let r2_term = (r * r) * p;
    let exp_r = 1.0 + (r + r2_term);
    scale_pow2(exp_r, k)
}

/// Deterministic natural logarithm `ln(x)` using IEEE-754 mantissa extraction
/// onto `[sqrt(1/2), sqrt(2)]` and the odd-power atanh series
/// `ln(m) = 2 * sum_{j=0}^{9} u^{2j+1} / (2j+1)` where `u = (m - 1)/(m + 1)`.
///
/// Relative error is `< 1e-15` for all positive finite `f64` values.
pub fn det_ln(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    if x == 1.0 {
        return 0.0;
    }

    let mut bits = x.to_bits();
    let mut exp_bits = ((bits >> 52) & 0x7ff) as i64;
    let mut k = if exp_bits == 0 {
        let norm = x * TWO_POW_54;
        bits = norm.to_bits();
        exp_bits = ((bits >> 52) & 0x7ff) as i64;
        exp_bits - 1023 - 54
    } else {
        exp_bits - 1023
    };

    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023_u64 << 52));
    if m > SQRT2 {
        m *= 0.5;
        k += 1;
    }

    let u = (m - 1.0) / (m + 1.0);
    let u2 = u * u;

    // 2 * u * (1 + u^2/3 + ... + u^18/19)
    let mut p = 1.0 / 19.0;
    p = (1.0 / 17.0) + u2 * p;
    p = (1.0 / 15.0) + u2 * p;
    p = (1.0 / 13.0) + u2 * p;
    p = (1.0 / 11.0) + u2 * p;
    p = (1.0 / 9.0) + u2 * p;
    p = (1.0 / 7.0) + u2 * p;
    p = (1.0 / 5.0) + u2 * p;
    p = (1.0 / 3.0) + u2 * p;
    p = 1.0 + u2 * p;

    let ln_m = (2.0 * u) * p;
    let kf = k as f64;
    let hi = kf * LN2_HI;
    let lo = kf * LN2_LO + ln_m;
    hi + lo
}

/// Deterministic `base^exp` built from [`det_exp`] and [`det_ln`] for positive bases.
pub fn det_pow(base: f64, exp: f64) -> f64 {
    if exp == 0.0 {
        return 1.0;
    }
    if base.is_nan() || exp.is_nan() {
        return f64::NAN;
    }
    if base == 1.0 {
        return 1.0;
    }
    if base > 0.0 {
        if base.is_infinite() {
            return if exp > 0.0 { f64::INFINITY } else { 0.0 };
        }
        return det_exp(exp * det_ln(base));
    }
    if base == 0.0 {
        return if exp > 0.0 { 0.0 } else { f64::INFINITY };
    }
    f64::NAN
}

/// Deterministic `asin(x)` on `[-1, 1]` via `trig::atan2(x, trig::sqrt(1 - x^2))`.
///
/// Clamps finite inputs into `[-1.0, 1.0]` before computing the radical so
/// 1-ULP rounding above `1.0` never triggers a negative `sqrt`.
#[inline]
pub fn det_asin_clamped(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let clamped = if x < -1.0 {
        -1.0
    } else if x > 1.0 {
        1.0
    } else if x == 0.0 {
        // Canonicalize -0.0 -> +0.0
        0.0
    } else {
        x
    };
    let rem = 1.0 - clamped * clamped;
    let cos_val = if rem <= 0.0 { 0.0 } else { trig::sqrt(rem) };
    trig::atan2(clamped, cos_val)
}
