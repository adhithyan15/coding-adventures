//! Arithmetic and rounding primitives.
//!
//! These mirror Excel's built-in arithmetic / rounding helpers and are
//! pure functions of their inputs. Each scalar function accepts an `f64`
//! and either returns an `f64` directly (total functions like `abs`, `sign`)
//! or a `Result<Number, MathError>` (functions with parameter constraints
//! such as `MROUND(x, 0)` being a domain error).
//!
//! NA handling: every function below treats the canonical `r-vector`
//! NA bit-pattern as a propagating input. If you pass NA in, you get NA out
//! (as `Number::Float(na_real())` for `Result`-returning functions).

use crate::{MathError, MathResult};
use numeric_tower::Number;
use r_vector::{is_na_real, na_real};

/// Excel `ABS(x)`. Absolute value. NA propagates.
///
/// ```text
/// abs(-3.0) = 3.0
/// abs(NaN)  = NaN
/// abs(NA)   = NA
/// ```
pub fn abs(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.abs()
}

/// Excel `SIGN(x)`. Returns `1` for positive, `-1` for negative, `0` for zero.
/// NaN -> NaN, NA -> NA. Excel's `SIGN(0) = 0` (we match).
pub fn sign(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    if x.is_nan() {
        return f64::NAN;
    }
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Excel `INT(x)`. Rounds *towards negative infinity* — i.e. floor.
/// Note this differs from `TRUNC`, which rounds towards zero.
///
/// ```text
/// int( 1.9) =  1.0
/// int(-1.1) = -2.0
/// ```
pub fn int(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.floor()
}

/// Excel `TRUNC(x, digits=0)`. Rounds *towards zero* to `digits` places.
/// `digits` may be negative (truncate to tens, hundreds, etc.).
///
/// ```text
/// trunc( 1.9, 0) =  1.0
/// trunc(-1.9, 0) = -1.0    (NOT -2.0 — sign-preserving)
/// trunc(123.456, 2) = 123.45
/// trunc(123.456,-1) = 120.0
/// ```
pub fn trunc(x: f64, digits: i32) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    if !x.is_finite() {
        return x;
    }
    let factor = 10f64.powi(digits);
    (x * factor).trunc() / factor
}

/// Excel `FLOOR.MATH(x, significance=1)` simplified to round-towards-minus-infinity
/// at a given step. The default step of `1.0` matches `INT(x)`. Sign of
/// significance is ignored (Excel `FLOOR.MATH` matches this behavior).
///
/// Returns `DomainError` if `significance == 0`.
pub fn floor(x: f64, significance: f64) -> MathResult<Number> {
    if is_na_real(x) || is_na_real(significance) {
        return Ok(Number::Float(na_real()));
    }
    if significance == 0.0 {
        // Excel: FLOOR with significance 0 yields #DIV/0!. We surface it as
        // DomainError because the operation has no defined value.
        return Err(MathError::DomainError {
            function: "floor",
            what: "significance must be non-zero".into(),
        });
    }
    let s = significance.abs();
    Ok(Number::Float((x / s).floor() * s))
}

/// Excel `CEILING.MATH(x, significance=1)`. Round towards positive infinity
/// at the chosen step. Sign of significance is ignored.
///
/// Returns `DomainError` if `significance == 0`.
pub fn ceiling(x: f64, significance: f64) -> MathResult<Number> {
    if is_na_real(x) || is_na_real(significance) {
        return Ok(Number::Float(na_real()));
    }
    if significance == 0.0 {
        return Err(MathError::DomainError {
            function: "ceiling",
            what: "significance must be non-zero".into(),
        });
    }
    let s = significance.abs();
    Ok(Number::Float((x / s).ceil() * s))
}

/// Excel `ROUND(x, digits)`. **Round-half-away-from-zero** at `digits`
/// decimal places (Excel's canonical behavior — NOT banker's rounding).
///
/// ```text
/// round( 0.5, 0) =  1.0   (away from zero)
/// round(-0.5, 0) = -1.0   (away from zero)
/// round( 1.25, 1) = 1.3
/// round(123.45,-1) = 120.0
/// ```
pub fn round(x: f64, digits: i32) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    if !x.is_finite() {
        return x;
    }
    let factor = 10f64.powi(digits);
    // f64::round is half-away-from-zero, which matches Excel.
    (x * factor).round() / factor
}

/// Excel `ROUNDUP(x, digits)`. Round *away from zero* to `digits` places.
/// `ROUNDUP(1.1, 0) = 2`, `ROUNDUP(-1.1, 0) = -2`.
pub fn roundup(x: f64, digits: i32) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    if !x.is_finite() {
        return x;
    }
    let factor = 10f64.powi(digits);
    let scaled = x * factor;
    let rounded = if scaled >= 0.0 {
        scaled.ceil()
    } else {
        scaled.floor()
    };
    rounded / factor
}

/// Excel `ROUNDDOWN(x, digits)`. Round *towards zero* — same as `TRUNC`.
pub fn rounddown(x: f64, digits: i32) -> f64 {
    trunc(x, digits)
}

/// Excel `MROUND(number, multiple)`. Round to the nearest multiple of
/// `multiple`. In Excel, `number` and `multiple` must share a sign or the
/// result is `#NUM!`; we mirror that with `DomainError`. `multiple == 0`
/// returns `0` (Excel's documented behavior).
pub fn mround(number: f64, multiple: f64) -> MathResult<Number> {
    if is_na_real(number) || is_na_real(multiple) {
        return Ok(Number::Float(na_real()));
    }
    if multiple == 0.0 {
        return Ok(Number::Float(0.0));
    }
    if number != 0.0 && number.signum() != multiple.signum() {
        return Err(MathError::DomainError {
            function: "mround",
            what: "number and multiple must share a sign".into(),
        });
    }
    Ok(Number::Float((number / multiple).round() * multiple))
}

/// Excel `EVEN(x)`. Rounds *away from zero* to the nearest even integer.
/// `EVEN(1.5) = 2`, `EVEN(3) = 4`, `EVEN(-1) = -2`, `EVEN(0) = 0`.
pub fn even(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    if !x.is_finite() {
        return x;
    }
    if x == 0.0 {
        return 0.0;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_ceiled = x.abs().ceil();
    // Bump to the next even integer if odd.
    let even_abs = if (abs_ceiled as i64) % 2 == 0 {
        abs_ceiled
    } else {
        abs_ceiled + 1.0
    };
    sign * even_abs
}

/// Excel `ODD(x)`. Rounds *away from zero* to the nearest odd integer.
/// `ODD(1.5) = 3`, `ODD(2) = 3`, `ODD(-1.5) = -3`, `ODD(0) = 1`.
pub fn odd(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    if !x.is_finite() {
        return x;
    }
    if x == 0.0 {
        return 1.0;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_ceiled = x.abs().ceil();
    let odd_abs = if (abs_ceiled as i64) % 2 == 1 {
        abs_ceiled
    } else {
        abs_ceiled + 1.0
    };
    sign * odd_abs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(n: Number) -> f64 {
        match n {
            Number::Float(v) => v,
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn abs_and_sign_basic() {
        assert_eq!(abs(-3.0), 3.0);
        assert_eq!(abs(3.0), 3.0);
        assert_eq!(sign(-2.0), -1.0);
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(sign(5.0), 1.0);
    }

    #[test]
    fn abs_propagates_na() {
        assert!(is_na_real(abs(na_real())));
        assert!(is_na_real(sign(na_real())));
    }

    #[test]
    fn int_floors_towards_minus_infinity() {
        assert_eq!(int(1.9), 1.0);
        assert_eq!(int(-1.1), -2.0);
        assert_eq!(int(0.0), 0.0);
    }

    #[test]
    fn trunc_is_sign_preserving() {
        assert_eq!(trunc(1.9, 0), 1.0);
        assert_eq!(trunc(-1.9, 0), -1.0);
        assert!((trunc(123.456, 2) - 123.45).abs() < 1e-9);
        assert_eq!(trunc(123.456, -1), 120.0);
    }

    #[test]
    fn floor_ceiling_handle_significance() {
        assert_eq!(extract(floor(7.3, 1.0).unwrap()), 7.0);
        assert_eq!(extract(floor(7.3, 2.0).unwrap()), 6.0);
        assert_eq!(extract(ceiling(7.3, 2.0).unwrap()), 8.0);
        // Sign of significance is ignored.
        assert_eq!(extract(floor(7.3, -2.0).unwrap()), 6.0);
    }

    #[test]
    fn floor_zero_significance_is_domain_error() {
        let err = floor(1.0, 0.0).unwrap_err();
        assert!(matches!(err, MathError::DomainError { .. }));
        let err = ceiling(1.0, 0.0).unwrap_err();
        assert!(matches!(err, MathError::DomainError { .. }));
    }

    #[test]
    fn round_is_half_away_from_zero() {
        assert_eq!(round(0.5, 0), 1.0);
        assert_eq!(round(-0.5, 0), -1.0);
        assert_eq!(round(1.5, 0), 2.0);
        assert_eq!(round(2.5, 0), 3.0); // NOT 2.0 (banker's would be 2)
        assert!((round(1.25, 1) - 1.3).abs() < 1e-9);
        assert_eq!(round(123.45, -1), 120.0);
    }

    #[test]
    fn roundup_and_rounddown() {
        assert_eq!(roundup(1.1, 0), 2.0);
        assert_eq!(roundup(-1.1, 0), -2.0);
        assert_eq!(rounddown(1.9, 0), 1.0);
        assert_eq!(rounddown(-1.9, 0), -1.0);
    }

    #[test]
    fn mround_matches_excel() {
        assert_eq!(extract(mround(10.0, 3.0).unwrap()), 9.0);
        assert_eq!(extract(mround(-10.0, -3.0).unwrap()), -9.0);
        // 0.2 * round(1.3/0.2) = 0.2 * 7 = 1.4 (within fp epsilon)
        assert!((extract(mround(1.3, 0.2).unwrap()) - 1.4).abs() < 1e-9);
        assert_eq!(extract(mround(5.0, 0.0).unwrap()), 0.0);
        assert!(matches!(
            mround(5.0, -2.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn even_and_odd_round_away_from_zero() {
        assert_eq!(even(1.5), 2.0);
        assert_eq!(even(3.0), 4.0);
        assert_eq!(even(-1.0), -2.0);
        assert_eq!(even(0.0), 0.0);
        assert_eq!(odd(1.5), 3.0);
        assert_eq!(odd(2.0), 3.0);
        assert_eq!(odd(-1.5), -3.0);
        assert_eq!(odd(0.0), 1.0);
    }

    #[test]
    fn rounding_propagates_na() {
        assert!(is_na_real(int(na_real())));
        assert!(is_na_real(trunc(na_real(), 2)));
        assert!(is_na_real(round(na_real(), 0)));
        assert!(is_na_real(roundup(na_real(), 0)));
        assert!(is_na_real(rounddown(na_real(), 0)));
        assert!(is_na_real(even(na_real())));
        assert!(is_na_real(odd(na_real())));
        match floor(na_real(), 1.0).unwrap() {
            Number::Float(v) => assert!(is_na_real(v)),
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn rounding_passes_through_non_finite() {
        assert!(trunc(f64::INFINITY, 0).is_infinite());
        assert!(round(f64::NEG_INFINITY, 0).is_infinite());
        assert!(roundup(f64::NAN, 0).is_nan());
    }
}
