//! Modular arithmetic and integer number theory.
//!
//! All four functions operate on `f64` values that must represent integers
//! (or are converted via truncation, matching Excel). The internal arithmetic
//! is done in `i128` to keep the full Excel integer range (`+/- 2^53 ish`) safely.
//!
//! Excel parity:
//! * `MOD(n, d)` — sign follows the divisor (R `%%` matches).
//! * `QUOTIENT(n, d)` — integer division, sign-preserving (truncated towards zero).
//! * `GCD(a, b, ...)` — greatest common divisor. We expose the 2-arg form;
//!   the variadic form is left to a higher layer that builds the spreadsheet
//!   facade (a simple `fold` does the job).
//! * `LCM(a, b, ...)` — likewise.

use crate::{MathError, MathResult};
use numeric_tower::Number;
use r_vector::{is_na_real, na_real};

/// Excel `MOD(number, divisor)`. Result has the sign of the divisor.
///
/// `DivisionByZero` if `divisor == 0`. Inputs that are not finite produce
/// `NaN` (and we let it through).
///
/// ```text
/// mod( 7.0,  3.0) =  1.0
/// mod(-7.0,  3.0) =  2.0     (sign follows divisor)
/// mod( 7.0, -3.0) = -2.0
/// mod(-7.0, -3.0) = -1.0
/// ```
pub fn modulo(number: f64, divisor: f64) -> MathResult<Number> {
    if is_na_real(number) || is_na_real(divisor) {
        return Ok(Number::Float(na_real()));
    }
    if divisor == 0.0 {
        return Err(MathError::DivisionByZero { function: "mod" });
    }
    // The classic formula: m = n - d * floor(n/d) yields a result with the
    // sign of d (when d != 0). Rust's `%` is C-style (sign-of-dividend), so
    // we cannot just use `%`.
    let q = (number / divisor).floor();
    Ok(Number::Float(number - divisor * q))
}

/// Excel `QUOTIENT(numerator, denominator)`. Integer division, sign-preserving
/// (truncates towards zero).
///
/// `DivisionByZero` if `denominator == 0`.
///
/// ```text
/// quotient( 7.0,  3.0) =  2
/// quotient(-7.0,  3.0) = -2  (truncates towards zero, NOT -3)
/// ```
pub fn quotient(numerator: f64, denominator: f64) -> MathResult<Number> {
    if is_na_real(numerator) || is_na_real(denominator) {
        return Ok(Number::Float(na_real()));
    }
    if denominator == 0.0 {
        return Err(MathError::DivisionByZero {
            function: "quotient",
        });
    }
    Ok(Number::Float((numerator / denominator).trunc()))
}

/// Excel `GCD(a, b)`. Greatest common divisor of two non-negative integers
/// (Excel coerces fractional inputs by truncation; negatives are rejected).
///
/// `DomainError` if either input is negative or not an integer value, or if
/// the magnitude exceeds the `i64` range.
pub fn gcd(a: f64, b: f64) -> MathResult<Number> {
    if is_na_real(a) || is_na_real(b) {
        return Ok(Number::Float(na_real()));
    }
    let ai = coerce_nonneg_int("gcd", a)?;
    let bi = coerce_nonneg_int("gcd", b)?;
    Ok(Number::Float(gcd_u64(ai, bi) as f64))
}

/// Excel `LCM(a, b)`. Least common multiple. `LCM(0, x) = 0`. `DomainError`
/// for negative inputs; `Overflow` if the result would exceed `i64::MAX`.
pub fn lcm(a: f64, b: f64) -> MathResult<Number> {
    if is_na_real(a) || is_na_real(b) {
        return Ok(Number::Float(na_real()));
    }
    let ai = coerce_nonneg_int("lcm", a)?;
    let bi = coerce_nonneg_int("lcm", b)?;
    if ai == 0 || bi == 0 {
        return Ok(Number::Float(0.0));
    }
    let g = gcd_u64(ai, bi);
    // (a / g) * b cannot overflow u128 because a,b fit in u64.
    let product = (ai as u128 / g as u128) * bi as u128;
    if product > i64::MAX as u128 {
        return Err(MathError::Overflow { function: "lcm" });
    }
    Ok(Number::Float(product as f64))
}

// --- helpers ---

fn coerce_nonneg_int(function: &'static str, value: f64) -> MathResult<u64> {
    if !value.is_finite() {
        return Err(MathError::DomainError {
            function,
            what: format!("expected a non-negative integer, got {value}"),
        });
    }
    if value < 0.0 {
        return Err(MathError::DomainError {
            function,
            what: format!("expected a non-negative integer, got {value}"),
        });
    }
    let truncated = value.trunc();
    if truncated != value {
        return Err(MathError::DomainError {
            function,
            what: format!("expected an integer value, got {value}"),
        });
    }
    if truncated > i64::MAX as f64 {
        return Err(MathError::Overflow { function });
    }
    Ok(truncated as u64)
}

/// Euclidean GCD on u64. `gcd(x, 0) = x` by convention.
fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
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
    fn mod_sign_follows_divisor() {
        assert_eq!(extract(modulo(7.0, 3.0).unwrap()), 1.0);
        assert_eq!(extract(modulo(-7.0, 3.0).unwrap()), 2.0);
        assert_eq!(extract(modulo(7.0, -3.0).unwrap()), -2.0);
        assert_eq!(extract(modulo(-7.0, -3.0).unwrap()), -1.0);
        // Excel: MOD(-3, 2) = 1
        assert_eq!(extract(modulo(-3.0, 2.0).unwrap()), 1.0);
    }

    #[test]
    fn mod_div_zero() {
        assert!(matches!(
            modulo(1.0, 0.0).unwrap_err(),
            MathError::DivisionByZero { .. }
        ));
    }

    #[test]
    fn quotient_truncates_towards_zero() {
        assert_eq!(extract(quotient(7.0, 3.0).unwrap()), 2.0);
        assert_eq!(extract(quotient(-7.0, 3.0).unwrap()), -2.0);
        assert_eq!(extract(quotient(7.0, -3.0).unwrap()), -2.0);
        assert!(matches!(
            quotient(1.0, 0.0).unwrap_err(),
            MathError::DivisionByZero { .. }
        ));
    }

    #[test]
    fn gcd_basic() {
        assert_eq!(extract(gcd(12.0, 18.0).unwrap()), 6.0);
        assert_eq!(extract(gcd(0.0, 5.0).unwrap()), 5.0);
        assert_eq!(extract(gcd(5.0, 0.0).unwrap()), 5.0);
        assert_eq!(extract(gcd(0.0, 0.0).unwrap()), 0.0);
        assert_eq!(extract(gcd(17.0, 13.0).unwrap()), 1.0);
    }

    #[test]
    fn gcd_rejects_negative_or_fractional() {
        assert!(matches!(
            gcd(-1.0, 2.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            gcd(1.5, 2.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn lcm_basic() {
        assert_eq!(extract(lcm(4.0, 6.0).unwrap()), 12.0);
        assert_eq!(extract(lcm(0.0, 5.0).unwrap()), 0.0);
        assert_eq!(extract(lcm(1.0, 1.0).unwrap()), 1.0);
        assert_eq!(extract(lcm(7.0, 11.0).unwrap()), 77.0);
    }

    #[test]
    fn modular_na_propagates() {
        let na_ok = |n: Number| matches!(n, Number::Float(v) if is_na_real(v));
        assert!(na_ok(modulo(na_real(), 1.0).unwrap()));
        assert!(na_ok(quotient(na_real(), 1.0).unwrap()));
        assert!(na_ok(gcd(na_real(), 1.0).unwrap()));
        assert!(na_ok(lcm(1.0, na_real()).unwrap()));
    }
}
