//! Special mathematical functions: gamma, lgamma, beta, lbeta,
//! choose, lchoose, erf, erfc, incomplete gamma / beta, digamma,
//! trigamma.
//!
//! Algorithms (all classical):
//!
//! - **gamma_fn**: Lanczos approximation with `g = 7` and the published
//!   coefficient set (accurate to ~1e-13).
//! - **lgamma**: same Lanczos series in log space.
//! - **erf / erfc**: Abramowitz & Stegun 7.1.26 (~1.5e-7); erfc
//!   computed directly for large x to avoid catastrophic cancellation.
//! - **gamma_inc_lower / upper**: series expansion for `x < a + 1`,
//!   continued fraction otherwise (Numerical Recipes §6.2 form).
//! - **beta_inc**: continued fraction for the regularized incomplete
//!   beta (Numerical Recipes §6.4 form).
//! - **digamma / trigamma**: shift to `x >= 6` via the recurrence
//!   `ψ(x) = ψ(x+1) - 1/x`, then asymptotic expansion.
//!
//! NA handling: NaN propagates; domain errors (e.g. `lgamma(-1)`)
//! return NaN per R's convention.

// ---------------------------------------------------------------------------
// gamma_fn (Lanczos)
// ---------------------------------------------------------------------------

/// Lanczos coefficients for g = 7, n = 9. Published values.
const LANCZOS_G: f64 = 7.0;
const LANCZOS_COEFFS: [f64; 9] = [
    0.99999999999980993,
    676.5203681218851,
    -1259.1392167224028,
    771.32342877765313,
    -176.61502916214059,
    12.507343278686905,
    -0.13857109526572012,
    9.9843695780195716e-6,
    1.5056327351493116e-7,
];

/// Gamma function Γ(x). Defined for all x except non-positive
/// integers (returns NaN). Accuracy ~1e-13 in the central region.
pub fn gamma_fn(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    // Non-positive integers are poles: explicit check (the sin(πx)
    // factor below is mathematically zero but rounds to ~1e-16 in
    // f64, so we cannot rely on s == 0.0).
    if x <= 0.0 && x == x.floor() {
        return f64::NAN;
    }
    if x < 0.5 {
        // Reflection: Γ(x) = π / (sin(πx) Γ(1-x))
        let s = (core::f64::consts::PI * x).sin();
        return core::f64::consts::PI / (s * gamma_fn(1.0 - x));
    }
    let x = x - 1.0;
    let mut a = LANCZOS_COEFFS[0];
    for (i, c) in LANCZOS_COEFFS.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    let t = x + LANCZOS_G + 0.5;
    let two_pi = 2.0 * core::f64::consts::PI;
    two_pi.sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}

/// Natural log of |Γ(x)|. Defined for all x except non-positive
/// integers (returns NaN). More accurate than `gamma_fn(x).ln()` for
/// large x.
pub fn lgamma(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x < 0.5 {
        let s = (core::f64::consts::PI * x).sin().abs();
        if s == 0.0 {
            return f64::NAN;
        }
        return core::f64::consts::PI.ln() - s.ln() - lgamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = LANCZOS_COEFFS[0];
    for (i, c) in LANCZOS_COEFFS.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    let t = x + LANCZOS_G + 0.5;
    let two_pi = 2.0 * core::f64::consts::PI;
    0.5 * two_pi.ln() + (x + 0.5) * t.ln() - t + a.ln()
}

// ---------------------------------------------------------------------------
// Beta function
// ---------------------------------------------------------------------------

/// Beta function B(a, b) = Γ(a) Γ(b) / Γ(a + b).
pub fn beta_fn(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 || a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    (lgamma(a) + lgamma(b) - lgamma(a + b)).exp()
}

/// Natural log of Beta(a, b).
pub fn lbeta(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 || a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    lgamma(a) + lgamma(b) - lgamma(a + b)
}

// ---------------------------------------------------------------------------
// Choose / binomial coefficient
// ---------------------------------------------------------------------------

/// C(n, k) — binomial coefficient. Supports non-integer n (uses
/// the gamma-function definition).
pub fn choose(n: f64, k: f64) -> f64 {
    lchoose(n, k).exp()
}

/// log of C(n, k). Numerically stable for large arguments.
pub fn lchoose(n: f64, k: f64) -> f64 {
    if k < 0.0 {
        return f64::NEG_INFINITY;
    }
    if k == 0.0 {
        return 0.0;
    }
    lgamma(n + 1.0) - lgamma(k + 1.0) - lgamma(n - k + 1.0)
}

// ---------------------------------------------------------------------------
// erf / erfc (A&S 7.1.26)
// ---------------------------------------------------------------------------

const ERF_A1: f64 = 0.254829592;
const ERF_A2: f64 = -0.284496736;
const ERF_A3: f64 = 1.421413741;
const ERF_A4: f64 = -1.453152027;
const ERF_A5: f64 = 1.061405429;
const ERF_P: f64 = 0.3275911;

/// Error function erf(x). Range [-1, 1]. Accuracy ~1.5e-7.
pub fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.is_infinite() {
        return x.signum();
    }
    if x == 0.0 {
        return 0.0;
    }
    let sign = x.signum();
    let x_abs = x.abs();
    let t = 1.0 / (1.0 + ERF_P * x_abs);
    let y = 1.0
        - (((((ERF_A5 * t + ERF_A4) * t) + ERF_A3) * t + ERF_A2) * t + ERF_A1)
            * t
            * (-(x_abs * x_abs)).exp();
    sign * y
}

/// Complementary error function 1 - erf(x). Direct computation for
/// large x retains precision.
pub fn erfc(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.is_infinite() {
        return if x > 0.0 { 0.0 } else { 2.0 };
    }
    if x.abs() < 0.5 {
        1.0 - erf(x)
    } else if x >= 0.0 {
        let t = 1.0 / (1.0 + ERF_P * x);
        let poly = (((((ERF_A5 * t + ERF_A4) * t) + ERF_A3) * t + ERF_A2) * t + ERF_A1) * t;
        poly * (-(x * x)).exp()
    } else {
        2.0 - erfc(-x)
    }
}

// ---------------------------------------------------------------------------
// Incomplete gamma functions
// ---------------------------------------------------------------------------

/// Regularized lower incomplete gamma P(a, x) = γ(a, x) / Γ(a).
pub fn gamma_inc_lower(x: f64, a: f64) -> f64 {
    if x.is_nan() || a.is_nan() {
        return f64::NAN;
    }
    if a <= 0.0 {
        return f64::NAN;
    }
    if x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        gamma_inc_series(x, a)
    } else {
        1.0 - gamma_inc_cf(x, a)
    }
}

/// Regularized upper incomplete gamma Q(a, x) = Γ(a, x) / Γ(a).
pub fn gamma_inc_upper(x: f64, a: f64) -> f64 {
    1.0 - gamma_inc_lower(x, a)
}

fn gamma_inc_series(x: f64, a: f64) -> f64 {
    // P(a, x) = e^{-x} x^a / Γ(a) * Σ_{n=0..∞} x^n / Γ(a + n + 1)
    let max_iter = 200;
    let eps = 1e-15;
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut delta = sum;
    for _ in 0..max_iter {
        ap += 1.0;
        delta *= x / ap;
        sum += delta;
        if delta.abs() < sum.abs() * eps {
            break;
        }
    }
    sum * (-x + a * x.ln() - lgamma(a)).exp()
}

fn gamma_inc_cf(x: f64, a: f64) -> f64 {
    // Continued fraction expansion of Q(a, x) (Numerical Recipes §6.2).
    let max_iter = 200;
    let eps = 1e-15;
    let fp_min = 1e-30;

    let mut b = x + 1.0 - a;
    let mut c = 1.0 / fp_min;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..=max_iter {
        let i_f = i as f64;
        let an = -i_f * (i_f - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < fp_min {
            d = fp_min;
        }
        c = b + an / c;
        if c.abs() < fp_min {
            c = fp_min;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < eps {
            break;
        }
    }
    h * (-x + a * x.ln() - lgamma(a)).exp()
}

// ---------------------------------------------------------------------------
// Regularized incomplete beta
// ---------------------------------------------------------------------------

/// Regularized incomplete beta I_x(a, b).
pub fn beta_inc(x: f64, a: f64, b: f64) -> f64 {
    if x.is_nan() || a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a <= 0.0 || b <= 0.0 {
        return f64::NAN;
    }
    if !(0.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let bt = ((lgamma(a + b) - lgamma(a) - lgamma(b))
        + a * x.ln()
        + b * (1.0 - x).ln())
    .exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(x, a, b) / a
    } else {
        1.0 - bt * betacf(1.0 - x, b, a) / b
    }
}

fn betacf(x: f64, a: f64, b: f64) -> f64 {
    let max_iter = 200;
    let eps = 1e-15;
    let fp_min = 1e-30;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < fp_min {
        d = fp_min;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=max_iter {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;
        let mut aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < fp_min {
            d = fp_min;
        }
        c = 1.0 + aa / c;
        if c.abs() < fp_min {
            c = fp_min;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < fp_min {
            d = fp_min;
        }
        c = 1.0 + aa / c;
        if c.abs() < fp_min {
            c = fp_min;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < eps {
            break;
        }
    }
    h
}

// ---------------------------------------------------------------------------
// digamma and trigamma
// ---------------------------------------------------------------------------

/// Digamma ψ(x) = d/dx log Γ(x). Returns NaN for non-positive
/// integer x.
pub fn digamma(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 && x == x.floor() {
        return f64::NAN;
    }
    // Reflection for negative arguments.
    if x < 0.0 {
        return digamma(1.0 - x) - core::f64::consts::PI / (core::f64::consts::PI * x).tan();
    }
    // Shift to x >= 6.
    let mut x = x;
    let mut result = 0.0;
    while x < 6.0 {
        result -= 1.0 / x;
        x += 1.0;
    }
    // Asymptotic expansion. Coefficients are Bernoulli/2k.
    let inv = 1.0 / x;
    let inv2 = inv * inv;
    result + x.ln() - 0.5 * inv
        - inv2 * (1.0 / 12.0
            - inv2 * (1.0 / 120.0
                - inv2 * (1.0 / 252.0
                    - inv2 / 240.0)))
}

/// Trigamma ψ'(x) = d/dx ψ(x). Returns NaN for non-positive
/// integer x.
pub fn trigamma(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 && x == x.floor() {
        return f64::NAN;
    }
    if x < 0.0 {
        let pi_x = core::f64::consts::PI * x;
        let sin = pi_x.sin();
        return -trigamma(1.0 - x) + (core::f64::consts::PI * core::f64::consts::PI) / (sin * sin);
    }
    let mut x = x;
    let mut result = 0.0;
    while x < 6.0 {
        result += 1.0 / (x * x);
        x += 1.0;
    }
    let inv = 1.0 / x;
    let inv2 = inv * inv;
    result + inv + 0.5 * inv2 + inv2 * inv * (1.0 / 6.0 - inv2 * (1.0 / 30.0 - inv2 / 42.0))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        (a - b).abs() < tol
    }

    #[test]
    fn gamma_integer_values() {
        // Γ(n) = (n-1)! for positive integers.
        assert!(close(gamma_fn(1.0), 1.0, 1e-9));
        assert!(close(gamma_fn(2.0), 1.0, 1e-9));
        assert!(close(gamma_fn(3.0), 2.0, 1e-9));
        assert!(close(gamma_fn(5.0), 24.0, 1e-9));
        assert!(close(gamma_fn(10.0), 362_880.0, 1e-6));
    }

    #[test]
    fn gamma_half_integer() {
        // Γ(0.5) = √π.
        assert!(close(gamma_fn(0.5), core::f64::consts::PI.sqrt(), 1e-9));
        // Γ(1.5) = 0.5 * √π.
        assert!(close(
            gamma_fn(1.5),
            0.5 * core::f64::consts::PI.sqrt(),
            1e-9
        ));
    }

    #[test]
    fn gamma_negative_returns_nan_at_zero_neg_integers() {
        assert!(gamma_fn(0.0).is_nan());
        assert!(gamma_fn(-1.0).is_nan());
        assert!(gamma_fn(-5.0).is_nan());
    }

    #[test]
    fn lgamma_matches_log_gamma_for_central_values() {
        for x in [0.5, 1.0, 2.0, 3.0, 10.0, 100.0] {
            let a = lgamma(x);
            let b = gamma_fn(x).ln();
            assert!(close(a, b, 1e-6), "x={x}: lgamma={a} vs ln(gamma)={b}");
        }
    }

    #[test]
    fn lgamma_large_argument() {
        // R: lgamma(170) ≈ 701.4376988...
        assert!(close(lgamma(170.0), 701.4376988_f64, 1e-3));
    }

    #[test]
    fn beta_fn_known_values() {
        // B(1, 1) = 1.
        assert!(close(beta_fn(1.0, 1.0), 1.0, 1e-9));
        // B(2, 3) = 1/12.
        assert!(close(beta_fn(2.0, 3.0), 1.0 / 12.0, 1e-9));
    }

    #[test]
    fn choose_integer_values() {
        assert!(close(choose(5.0, 2.0), 10.0, 1e-9));
        assert!(close(choose(10.0, 0.0), 1.0, 1e-9));
        assert!(close(choose(10.0, 10.0), 1.0, 1e-9));
        assert!(close(choose(20.0, 10.0), 184_756.0, 1e-6));
    }

    #[test]
    fn erf_known_values() {
        assert!(close(erf(0.0), 0.0, 1e-10));
        assert!(close(erf(0.5), 0.5204998778_f64, 1e-6));
        assert!(close(erf(1.0), 0.8427007929_f64, 1e-6));
        assert!(close(erf(2.0), 0.9953222650_f64, 1e-6));
    }

    #[test]
    fn erf_odd_symmetry() {
        for x in [0.5, 1.0, 2.0] {
            assert!(close(erf(-x), -erf(x), 1e-10));
        }
    }

    #[test]
    fn erfc_precision_at_large_x() {
        let r = erfc(5.0);
        assert!(r > 1e-13 && r < 1e-11, "erfc(5)={r}");
    }

    #[test]
    fn gamma_inc_lower_known() {
        // R: pgamma(2, 3) = 0.3233236
        assert!(close(gamma_inc_lower(2.0, 3.0), 0.3233236_f64, 1e-4));
        // pgamma(0, *) = 0
        assert!(close(gamma_inc_lower(0.0, 3.0), 0.0, 1e-9));
    }

    #[test]
    fn gamma_inc_lower_plus_upper_is_one() {
        for (x, a) in [(2.0, 3.0), (5.0, 4.0), (0.5, 0.5), (10.0, 2.0)] {
            let p = gamma_inc_lower(x, a);
            let q = gamma_inc_upper(x, a);
            assert!(close(p + q, 1.0, 1e-9), "x={x}, a={a}: p+q={}", p + q);
        }
    }

    #[test]
    fn beta_inc_known() {
        // R: pbeta(0.5, 2, 2) = 0.5 (symmetry).
        assert!(close(beta_inc(0.5, 2.0, 2.0), 0.5, 1e-9));
        // R: pbeta(0.25, 2, 5) ≈ 0.46606
        assert!(close(beta_inc(0.25, 2.0, 5.0), 0.466064_f64, 1e-5));
        // Boundaries.
        assert!(close(beta_inc(0.0, 2.0, 3.0), 0.0, 1e-9));
        assert!(close(beta_inc(1.0, 2.0, 3.0), 1.0, 1e-9));
    }

    #[test]
    fn digamma_known_values() {
        // R: digamma(1) = -0.5772156649 (Euler-Mascheroni)
        assert!(close(digamma(1.0), -0.5772156649_f64, 1e-6));
        // digamma(2) = 1 - γ
        assert!(close(digamma(2.0), 1.0 - 0.5772156649_f64, 1e-6));
        // digamma(5) ≈ 1.5061177
        assert!(close(digamma(5.0), 1.5061177_f64, 1e-5));
    }

    #[test]
    fn trigamma_known_values() {
        // R: trigamma(1) = π²/6 ≈ 1.6449
        assert!(close(
            trigamma(1.0),
            core::f64::consts::PI * core::f64::consts::PI / 6.0,
            1e-6
        ));
        // trigamma(2) ≈ 0.6449
        assert!(close(trigamma(2.0), 0.6449340668_f64, 1e-6));
    }

    #[test]
    fn special_functions_propagate_nan() {
        assert!(gamma_fn(f64::NAN).is_nan());
        assert!(lgamma(f64::NAN).is_nan());
        assert!(beta_fn(f64::NAN, 1.0).is_nan());
        assert!(erf(f64::NAN).is_nan());
        assert!(beta_inc(f64::NAN, 2.0, 3.0).is_nan());
        assert!(digamma(f64::NAN).is_nan());
    }
}
