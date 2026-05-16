//! Trigonometric, inverse trig, and hyperbolic functions.
//!
//! All functions take angles in **radians** (matching Excel and R). Use
//! [`crate::conversion::degrees`] / [`crate::conversion::radians`] to convert
//! between radians and degrees.
//!
//! Excel parity:
//! * `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`
//! * `SINH`, `COSH`, `TANH`, `ASINH`, `ACOSH`, `ATANH`
//! * `SEC`, `CSC`, `COT`, plus their hyperbolic forms (we expose the basic
//!   reciprocal forms; hyperbolic reciprocals follow trivially)
//!
//! Domain rules:
//! * `asin`, `acos`: argument must lie in `[-1, 1]`.
//! * `acosh`: argument must be `>= 1`.
//! * `atanh`: argument must lie in `(-1, 1)`.
//! * `sec`, `csc`, `cot`: undefined where the denominator is zero — we let
//!   the IEEE 754 result through (`+inf`, `-inf`, or `NaN`) so the caller's
//!   spreadsheet semantics can decide.

use crate::{MathError, MathResult};
use numeric_tower::Number;
use r_vector::{is_na_real, na_real};

/// Excel `SIN(x)`. x in radians.
pub fn sin(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.sin()
}

/// Excel `COS(x)`. x in radians.
pub fn cos(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.cos()
}

/// Excel `TAN(x)`. x in radians. Returns `+inf`/`-inf` near asymptotes.
pub fn tan(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.tan()
}

/// Excel `ASIN(x)`. Result in `[-pi/2, pi/2]`. `DomainError` if `|x| > 1`.
pub fn asin(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if !(-1.0..=1.0).contains(&x) {
        return Err(MathError::DomainError {
            function: "asin",
            what: "argument must lie in [-1, 1]".into(),
        });
    }
    Ok(Number::Float(x.asin()))
}

/// Excel `ACOS(x)`. Result in `[0, pi]`. `DomainError` if `|x| > 1`.
pub fn acos(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if !(-1.0..=1.0).contains(&x) {
        return Err(MathError::DomainError {
            function: "acos",
            what: "argument must lie in [-1, 1]".into(),
        });
    }
    Ok(Number::Float(x.acos()))
}

/// Excel `ATAN(x)`. Result in `(-pi/2, pi/2)`. Total over all f64.
pub fn atan(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.atan()
}

/// Excel `ATAN2(y, x)`. **Note Excel's parameter order is `(x, y)`** but R
/// and most languages use `(y, x)`. We follow R / `f64::atan2` — caller is
/// responsible for flipping if mimicking Excel exactly. Documented here so
/// the spreadsheet adapter at the next layer up can swap arguments.
///
/// Returns 0 when both arguments are 0 (IEEE 754 convention).
pub fn atan2(y: f64, x: f64) -> f64 {
    if is_na_real(y) || is_na_real(x) {
        return na_real();
    }
    y.atan2(x)
}

/// Excel `SINH(x)`. Hyperbolic sine.
pub fn sinh(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.sinh()
}

/// Excel `COSH(x)`. Hyperbolic cosine.
pub fn cosh(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.cosh()
}

/// Excel `TANH(x)`. Hyperbolic tangent.
pub fn tanh(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.tanh()
}

/// Excel `ASINH(x)`. Inverse hyperbolic sine. Total over all f64.
pub fn asinh(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.asinh()
}

/// Excel `ACOSH(x)`. `DomainError` if `x < 1`.
pub fn acosh(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if x < 1.0 {
        return Err(MathError::DomainError {
            function: "acosh",
            what: "argument must be >= 1".into(),
        });
    }
    Ok(Number::Float(x.acosh()))
}

/// Excel `ATANH(x)`. `DomainError` if `|x| >= 1`.
pub fn atanh(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if !(-1.0 < x && x < 1.0) {
        return Err(MathError::DomainError {
            function: "atanh",
            what: "argument must lie in (-1, 1)".into(),
        });
    }
    Ok(Number::Float(x.atanh()))
}

/// Excel `SEC(x)`. Secant = 1 / cos(x). Returns `+inf` at cos zeros.
pub fn sec(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    1.0 / x.cos()
}

/// Excel `CSC(x)`. Cosecant = 1 / sin(x).
pub fn csc(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    1.0 / x.sin()
}

/// Excel `COT(x)`. Cotangent = cos(x) / sin(x). At sin zeros, returns the
/// IEEE 754 result (`+inf` or `-inf`).
pub fn cot(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.cos() / x.sin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PI;

    fn extract(n: Number) -> f64 {
        match n {
            Number::Float(v) => v,
            other => panic!("expected float, got {other:?}"),
        }
    }

    fn approx(a: f64, b: f64) {
        assert!(
            (a - b).abs() <= 1e-10,
            "expected {b}, got {a} (diff {:e})",
            (a - b).abs()
        );
    }

    #[test]
    fn sin_cos_at_known_angles() {
        approx(sin(0.0), 0.0);
        approx(cos(0.0), 1.0);
        approx(sin(PI / 2.0), 1.0);
        approx(cos(PI / 2.0), 0.0);
        approx(sin(PI), 0.0);
        approx(cos(PI), -1.0);
    }

    #[test]
    fn sin_squared_plus_cos_squared_is_one() {
        for &x in &[0.1, 0.5, 1.0, 2.0, 5.0, -3.7] {
            approx(sin(x).powi(2) + cos(x).powi(2), 1.0);
        }
    }

    #[test]
    fn tan_pi_over_four_is_one() {
        approx(tan(PI / 4.0), 1.0);
    }

    #[test]
    fn inverse_trig_domain() {
        approx(extract(asin(1.0).unwrap()), PI / 2.0);
        approx(extract(acos(0.0).unwrap()), PI / 2.0);
        approx(atan(1.0), PI / 4.0);
        assert!(matches!(
            asin(1.5).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            acos(-2.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn atan2_quadrants() {
        approx(atan2(1.0, 1.0), PI / 4.0);
        approx(atan2(1.0, -1.0), 3.0 * PI / 4.0);
        approx(atan2(-1.0, -1.0), -3.0 * PI / 4.0);
        approx(atan2(0.0, 0.0), 0.0);
    }

    #[test]
    fn hyperbolic_functions() {
        approx(sinh(0.0), 0.0);
        approx(cosh(0.0), 1.0);
        approx(tanh(0.0), 0.0);
        // cosh^2 - sinh^2 = 1 identity
        for &x in &[0.1, 1.0, 2.5] {
            approx(cosh(x).powi(2) - sinh(x).powi(2), 1.0);
        }
    }

    #[test]
    fn asinh_acosh_atanh() {
        approx(asinh(0.0), 0.0);
        approx(extract(acosh(1.0).unwrap()), 0.0);
        approx(extract(atanh(0.0).unwrap()), 0.0);
        assert!(matches!(
            acosh(0.5).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            atanh(1.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            atanh(-1.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn reciprocal_trig() {
        approx(sec(0.0), 1.0);
        approx(csc(PI / 2.0), 1.0);
        approx(cot(PI / 4.0), 1.0);
    }

    #[test]
    fn na_propagates() {
        let na_ok = |n: Number| matches!(n, Number::Float(v) if is_na_real(v));
        assert!(is_na_real(sin(na_real())));
        assert!(is_na_real(cos(na_real())));
        assert!(is_na_real(tan(na_real())));
        assert!(is_na_real(atan(na_real())));
        assert!(is_na_real(atan2(na_real(), 1.0)));
        assert!(is_na_real(atan2(1.0, na_real())));
        assert!(is_na_real(sinh(na_real())));
        assert!(is_na_real(cosh(na_real())));
        assert!(is_na_real(tanh(na_real())));
        assert!(is_na_real(asinh(na_real())));
        assert!(is_na_real(sec(na_real())));
        assert!(is_na_real(csc(na_real())));
        assert!(is_na_real(cot(na_real())));
        assert!(na_ok(asin(na_real()).unwrap()));
        assert!(na_ok(acos(na_real()).unwrap()));
        assert!(na_ok(acosh(na_real()).unwrap()));
        assert!(na_ok(atanh(na_real()).unwrap()));
    }
}
