//! Power, exponential, and logarithm functions.
//!
//! Excel/R cross-reference:
//!
//! | this crate     | Excel       | R              | Notes                              |
//! |----------------|-------------|----------------|------------------------------------|
//! | `power(x, y)`  | `POWER`     | `x ^ y`        | `power(0, 0)` is `DomainError`     |
//! | `sqrt(x)`      | `SQRT`      | `sqrt`         |                                    |
//! | `exp(x)`       | `EXP`       | `exp`          |                                    |
//! | `ln(x)`        | `LN`        | `log`          | natural log                        |
//! | `log10(x)`     | `LOG` 1-arg | `log10`        | Excel `LOG(x)` is base 10          |
//! | `log2(x)`      | `LOG(x,2)`  | `log2`         |                                    |
//! | `log_base(x,b)`| `LOG(x,b)`  | `log(x, base)` | base-b log                         |

use crate::{MathError, MathResult};
use numeric_tower::Number;
use r_vector::{is_na_real, na_real};

/// Excel `POWER(x, y)`. Returns `x^y`.
///
/// Domain rules (matching Excel):
/// * `x == 0 && y == 0` -> `DomainError` (Excel `#NUM!`; the limit is
///   indeterminate, though IEEE 754 defines `pow(0,0)=1`).
/// * `x == 0 && y < 0`  -> `DivisionByZero`.
/// * `x < 0 && y` not integer -> `DomainError` (complex result not supported
///   in real-valued path; the complex rung would be needed).
pub fn power(x: f64, y: f64) -> MathResult<Number> {
    if is_na_real(x) || is_na_real(y) {
        return Ok(Number::Float(na_real()));
    }
    if x == 0.0 && y == 0.0 {
        return Err(MathError::DomainError {
            function: "power",
            what: "0^0 is indeterminate".into(),
        });
    }
    if x == 0.0 && y < 0.0 {
        return Err(MathError::DivisionByZero { function: "power" });
    }
    if x < 0.0 && y.fract() != 0.0 && y.is_finite() {
        return Err(MathError::DomainError {
            function: "power",
            what: "negative base with non-integer exponent yields a complex result".into(),
        });
    }
    Ok(Number::Float(x.powf(y)))
}

/// Excel `SQRT(x)`. `DomainError` for negative `x`.
pub fn sqrt(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if x < 0.0 {
        return Err(MathError::DomainError {
            function: "sqrt",
            what: "argument must be >= 0".into(),
        });
    }
    Ok(Number::Float(x.sqrt()))
}

/// Excel `EXP(x)`. e^x. Total over all finite f64; overflows to +inf.
pub fn exp(x: f64) -> f64 {
    if is_na_real(x) {
        return na_real();
    }
    x.exp()
}

/// Excel `LN(x)`, R `log(x)`. Natural logarithm.
/// `DomainError` for `x <= 0`.
pub fn ln(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if x <= 0.0 {
        return Err(MathError::DomainError {
            function: "ln",
            what: "argument must be > 0".into(),
        });
    }
    Ok(Number::Float(x.ln()))
}

/// Excel `LOG(x)` (1-arg), R `log10(x)`. Base-10 logarithm.
pub fn log10(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if x <= 0.0 {
        return Err(MathError::DomainError {
            function: "log10",
            what: "argument must be > 0".into(),
        });
    }
    Ok(Number::Float(x.log10()))
}

/// R `log2(x)`. Base-2 logarithm.
pub fn log2(x: f64) -> MathResult<Number> {
    if is_na_real(x) {
        return Ok(Number::Float(na_real()));
    }
    if x <= 0.0 {
        return Err(MathError::DomainError {
            function: "log2",
            what: "argument must be > 0".into(),
        });
    }
    Ok(Number::Float(x.log2()))
}

/// Excel `LOG(x, base)`, R `log(x, base)`. Logarithm of `x` in arbitrary base.
/// `DomainError` for `x <= 0`, `base <= 0`, or `base == 1`.
pub fn log_base(x: f64, base: f64) -> MathResult<Number> {
    if is_na_real(x) || is_na_real(base) {
        return Ok(Number::Float(na_real()));
    }
    if x <= 0.0 {
        return Err(MathError::DomainError {
            function: "log_base",
            what: "argument must be > 0".into(),
        });
    }
    if base <= 0.0 || base == 1.0 {
        return Err(MathError::DomainError {
            function: "log_base",
            what: "base must be > 0 and != 1".into(),
        });
    }
    Ok(Number::Float(x.log(base)))
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

    fn approx(a: f64, b: f64) {
        assert!(
            (a - b).abs() <= 1e-10 || (a - b).abs() / b.abs().max(1.0) <= 1e-10,
            "expected {b}, got {a}"
        );
    }

    #[test]
    fn power_basic() {
        approx(extract(power(2.0, 10.0).unwrap()), 1024.0);
        approx(extract(power(9.0, 0.5).unwrap()), 3.0);
        approx(extract(power(2.0, -1.0).unwrap()), 0.5);
        approx(extract(power(-2.0, 3.0).unwrap()), -8.0);
    }

    #[test]
    fn power_0_0_is_domain_error() {
        assert!(matches!(
            power(0.0, 0.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn power_0_negative_is_div_zero() {
        assert!(matches!(
            power(0.0, -1.0).unwrap_err(),
            MathError::DivisionByZero { .. }
        ));
    }

    #[test]
    fn power_negative_base_fractional_exp_is_domain_error() {
        assert!(matches!(
            power(-1.0, 0.5).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn sqrt_domain() {
        approx(extract(sqrt(4.0).unwrap()), 2.0);
        approx(extract(sqrt(0.0).unwrap()), 0.0);
        assert!(matches!(
            sqrt(-1.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn exp_ln_inverse() {
        for &x in &[0.1, 1.0, 2.5, 7.7] {
            let l = extract(ln(x).unwrap());
            approx(exp(l), x);
        }
    }

    #[test]
    fn ln_log10_log2_domain() {
        assert!(matches!(
            ln(0.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            ln(-1.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            log10(0.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            log2(-3.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn log_base_values() {
        approx(extract(log_base(1000.0, 10.0).unwrap()), 3.0);
        approx(extract(log_base(8.0, 2.0).unwrap()), 3.0);
        approx(extract(log10(100.0).unwrap()), 2.0);
        approx(extract(log2(32.0).unwrap()), 5.0);
    }

    #[test]
    fn log_base_bad_base() {
        assert!(matches!(
            log_base(5.0, 1.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            log_base(5.0, 0.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            log_base(5.0, -2.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn na_propagates() {
        let na_ok = |n: Number| matches!(n, Number::Float(v) if is_na_real(v));
        assert!(na_ok(power(na_real(), 2.0).unwrap()));
        assert!(na_ok(power(2.0, na_real()).unwrap()));
        assert!(na_ok(sqrt(na_real()).unwrap()));
        assert!(is_na_real(exp(na_real())));
        assert!(na_ok(ln(na_real()).unwrap()));
        assert!(na_ok(log10(na_real()).unwrap()));
        assert!(na_ok(log2(na_real()).unwrap()));
        assert!(na_ok(log_base(na_real(), 2.0).unwrap()));
    }
}
