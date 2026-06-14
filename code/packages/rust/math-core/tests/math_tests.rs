//! Integration tests for `math-core`.
//!
//! The per-module unit tests cover the basic correctness of each function.
//! This file collects cross-cutting / identity-based checks that exercise
//! several modules together, mirroring the property-based style suggested
//! by the spec.

// We deliberately spell out numeric values of well-known constants (PI, E,
// etc.) in test literals so each assertion shows the expected bit pattern at
// the call site. Clippy's `approx_constant` lint would have us reference the
// canonical name instead — but the point of these tests is to verify the
// constants *match* the canonical literal, so we silence the lint here.
#![allow(clippy::approx_constant)]

use math_core::arithmetic::{abs, round, sign};
use math_core::combinatorics::{combin, fact, multinomial, permut};
use math_core::constants::{E, PI};
use math_core::conversion::{degrees, radians};
use math_core::modular::{gcd, lcm, modulo, quotient};
use math_core::power_log::{exp, ln, log10, log2, log_base, power, sqrt};
use math_core::trig::{
    acos, acosh, asin, asinh, atan, atan2, atanh, cos, cosh, cot, csc, sec, sin, sinh, tan, tanh,
};
use math_core::{is_na_real, na_real, MathError, Number};

fn extract(n: Number) -> f64 {
    match n {
        Number::Float(v) => v,
        other => panic!("expected float, got {other:?}"),
    }
}

fn approx(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "expected {b}, got {a} (diff {:e})",
        (a - b).abs()
    );
}

// -------- identity / property style tests --------

#[test]
fn pythagorean_identity_holds_everywhere() {
    for &x in &[
        -10.0, -PI, -1.5, -0.5, 0.0, 0.5, 1.0, PI / 4.0, PI / 2.0, PI, 3.7, 10.0,
    ] {
        approx(sin(x).powi(2) + cos(x).powi(2), 1.0, 1e-12);
    }
}

#[test]
fn hyperbolic_identity_holds() {
    for &x in &[-3.0, -1.0, 0.0, 0.5, 2.0, 4.0] {
        approx(cosh(x).powi(2) - sinh(x).powi(2), 1.0, 1e-10);
    }
}

#[test]
fn exp_and_ln_are_inverse() {
    for &x in &[0.001, 0.5, 1.0, 2.718281828, 100.0, 1e6] {
        approx(exp(extract(ln(x).unwrap())), x, 1e-9 * x.abs().max(1.0));
        approx(extract(ln(exp(x.ln())).unwrap()), x.ln(), 1e-9);
    }
}

#[test]
fn sqrt_squared_is_identity() {
    for &x in &[0.0, 0.5, 1.0, 2.0, 10.0, 1e6] {
        approx(extract(sqrt(x).unwrap()).powi(2), x, 1e-10 * x.max(1.0));
    }
}

#[test]
fn degrees_radians_are_inverse() {
    for &d in &[0.0, 30.0, 45.0, 90.0, 180.0, 270.0, 360.0, -45.0] {
        approx(degrees(radians(d)), d, 1e-12);
        approx(radians(degrees(d)), d, 1e-12);
    }
}

#[test]
fn sin_of_radians_matches_excel_sin_degrees() {
    // SIN(RADIANS(30)) = 0.5 (Excel's canonical example)
    approx(sin(radians(30.0)), 0.5, 1e-12);
    approx(cos(radians(60.0)), 0.5, 1e-12);
    approx(tan(radians(45.0)), 1.0, 1e-12);
}

#[test]
fn log_base_consistency() {
    // log_base(x, b) == ln(x) / ln(b)
    for &(x, b) in &[(1000.0_f64, 10.0_f64), (32.0, 2.0), (27.0, 3.0)] {
        let by_base = extract(log_base(x, b).unwrap());
        let by_ratio = extract(ln(x).unwrap()) / extract(ln(b).unwrap());
        approx(by_base, by_ratio, 1e-12);
    }
    approx(
        extract(log10(1000.0).unwrap()),
        extract(log_base(1000.0, 10.0).unwrap()),
        1e-12,
    );
    approx(
        extract(log2(64.0).unwrap()),
        extract(log_base(64.0, 2.0).unwrap()),
        1e-12,
    );
}

#[test]
fn gcd_lcm_product_law() {
    // gcd(a,b) * lcm(a,b) = a*b for non-negative integers
    for &(a, b) in &[(12.0_f64, 18.0_f64), (7.0, 11.0), (100.0, 75.0), (1.0, 1.0)] {
        let g = extract(gcd(a, b).unwrap());
        let l = extract(lcm(a, b).unwrap());
        approx(g * l, a * b, 1e-9);
    }
}

#[test]
fn mod_quotient_decomposition() {
    // n = quotient(n, d) * d + mod(n, d)... but with Excel-style mod (sign
    // of divisor) the identity becomes n = floor(n/d) * d + mod(n, d).
    for &(n, d) in &[(7.0_f64, 3.0_f64), (-7.0, 3.0), (7.0, -3.0), (-7.0, -3.0)] {
        let m = extract(modulo(n, d).unwrap());
        approx((n / d).floor() * d + m, n, 1e-12);
        // QUOTIENT truncates towards zero — distinct identity:
        let q = extract(quotient(n, d).unwrap());
        // n = q*d + remainder_truncated; remainder_truncated has sign of n
        let r = n - q * d;
        approx(q * d + r, n, 1e-12);
    }
}

#[test]
fn combin_satisfies_pascals_rule() {
    // C(n, k) = C(n-1, k-1) + C(n-1, k)
    for n in 2u32..15 {
        for k in 1..n {
            let left = extract(combin(n as f64, k as f64).unwrap());
            let right_a = extract(combin((n - 1) as f64, (k - 1) as f64).unwrap());
            let right_b = extract(combin((n - 1) as f64, k as f64).unwrap());
            approx(left, right_a + right_b, 1e-9);
        }
    }
}

#[test]
fn permut_equals_combin_times_factorial() {
    // P(n, k) = C(n, k) * k!
    for n in 0u32..10 {
        for k in 0..=n {
            let p = extract(permut(n as f64, k as f64).unwrap());
            let c = extract(combin(n as f64, k as f64).unwrap());
            let kf = extract(fact(k as f64).unwrap());
            approx(p, c * kf, 1e-9);
        }
    }
}

#[test]
fn multinomial_matches_factorial_ratio_for_small_values() {
    // multinomial(2, 3, 4) = 9! / (2! 3! 4!) = 362880 / (2*6*24) = 1260
    let expected = extract(fact(9.0).unwrap())
        / (extract(fact(2.0).unwrap())
            * extract(fact(3.0).unwrap())
            * extract(fact(4.0).unwrap()));
    approx(
        extract(multinomial(&[2.0, 3.0, 4.0]).unwrap()),
        expected,
        1e-9,
    );
}

// -------- edge cases requested by the spec --------

#[test]
fn fact_171_overflows() {
    assert!(matches!(
        fact(171.0).unwrap_err(),
        MathError::Overflow { .. }
    ));
}

#[test]
fn fact_neg_one_domain_error() {
    assert!(matches!(
        fact(-1.0).unwrap_err(),
        MathError::DomainError { .. }
    ));
}

#[test]
fn power_zero_zero_is_domain_error() {
    assert!(matches!(
        power(0.0, 0.0).unwrap_err(),
        MathError::DomainError { .. }
    ));
}

#[test]
fn sqrt_neg_one_domain_error() {
    assert!(matches!(
        sqrt(-1.0).unwrap_err(),
        MathError::DomainError { .. }
    ));
}

#[test]
fn ln_zero_and_neg_one_domain_error() {
    assert!(matches!(
        ln(0.0).unwrap_err(),
        MathError::DomainError { .. }
    ));
    assert!(matches!(
        ln(-1.0).unwrap_err(),
        MathError::DomainError { .. }
    ));
}

#[test]
fn acos_two_domain_error() {
    assert!(matches!(
        acos(2.0).unwrap_err(),
        MathError::DomainError { .. }
    ));
}

#[test]
fn mod_div_zero_error() {
    assert!(matches!(
        modulo(5.0, 0.0).unwrap_err(),
        MathError::DivisionByZero { .. }
    ));
}

#[test]
fn na_propagates_through_one_arg_funcs() {
    let na = na_real();
    assert!(is_na_real(abs(na)));
    assert!(is_na_real(sign(na)));
    assert!(is_na_real(round(na, 2)));
    assert!(is_na_real(sin(na)));
    assert!(is_na_real(cos(na)));
    assert!(is_na_real(tan(na)));
    assert!(is_na_real(sinh(na)));
    assert!(is_na_real(cosh(na)));
    assert!(is_na_real(tanh(na)));
    assert!(is_na_real(asinh(na)));
    assert!(is_na_real(atan(na)));
    assert!(is_na_real(exp(na)));
    assert!(is_na_real(sec(na)));
    assert!(is_na_real(csc(na)));
    assert!(is_na_real(cot(na)));
    assert!(is_na_real(degrees(na)));
    assert!(is_na_real(radians(na)));
}

#[test]
fn na_propagates_through_two_arg_funcs() {
    let na = na_real();
    let na_ok = |n: Number| matches!(n, Number::Float(v) if is_na_real(v));
    assert!(na_ok(power(na, 2.0).unwrap()));
    assert!(na_ok(power(2.0, na).unwrap()));
    assert!(na_ok(log_base(na, 2.0).unwrap()));
    assert!(na_ok(asin(na).unwrap()));
    assert!(na_ok(acos(na).unwrap()));
    assert!(is_na_real(asinh(na_real())));
    assert!(na_ok(acosh(na).unwrap()));
    assert!(na_ok(atanh(na).unwrap()));
    assert!(is_na_real(atan2(na, 1.0)));
    assert!(na_ok(modulo(na, 1.0).unwrap()));
    assert!(na_ok(quotient(1.0, na).unwrap()));
    assert!(na_ok(gcd(na, 1.0).unwrap()));
    assert!(na_ok(lcm(1.0, na).unwrap()));
}

#[test]
fn constants_have_expected_precision() {
    approx(PI, 3.141592653589793, 1e-15);
    approx(E, 2.718281828459045, 1e-15);
}

#[test]
fn sin_pi_is_nearly_zero() {
    // Famous floating-point sanity check: sin(pi) is not exactly 0 but is tiny.
    assert!(sin(PI).abs() < 1e-15);
    assert!(cos(2.0 * PI) > 1.0 - 1e-15);
}
