//! Factorials, combinations, permutations, multinomials.
//!
//! Floating-point representable range:
//! * `FACT(n)`: `0 <= n <= 170`. `n = 170` is the largest factorial that
//!   fits in `f64`; `n = 171` overflows.
//! * `FACTDOUBLE(n)`: `n!!`. Bound is roughly `0 <= n <= 300`; we detect
//!   overflow at runtime.
//! * `COMBIN(n, k)` and `PERMUT(n, k)`: we compute via successive
//!   multiplication / division to stay accurate for moderate `n` and to
//!   support `n` larger than 170 when `k` is small.
//!
//! Excel parity:
//! * `FACT` (Excel name) -> `fact`
//! * `FACTDOUBLE` -> `fact_double`
//! * `COMBIN(n, k)` (without repetition) -> `combin`
//! * `COMBINA(n, k)` (with repetition) -> `combina`  (== `combin(n+k-1, k)`)
//! * `PERMUT(n, k)` (without repetition) -> `permut`
//! * `PERMUTATIONA(n, k)` (with repetition) -> `permuta` (== `n^k`)
//! * `MULTINOMIAL(k1, ..., kr)` -> `multinomial`

use crate::{MathError, MathResult};
use numeric_tower::Number;
use r_vector::{is_na_real, na_real};

const FACT_MAX: u32 = 170;

/// Excel `FACT(n)`. `n!`. `DomainError` for negative or fractional `n`,
/// `Overflow` for `n > 170`.
pub fn fact(n: f64) -> MathResult<Number> {
    if is_na_real(n) {
        return Ok(Number::Float(na_real()));
    }
    let k = coerce_nonneg_int("fact", n)?;
    if k > FACT_MAX as u64 {
        return Err(MathError::Overflow { function: "fact" });
    }
    let mut result = 1.0_f64;
    for i in 2..=k {
        result *= i as f64;
    }
    Ok(Number::Float(result))
}

/// Excel `FACTDOUBLE(n)`. Double factorial:
/// `n!! = n * (n-2) * (n-4) * ...` stopping at `1` (odd `n`) or `2` (even `n`).
/// By convention `0!! = 1` and `(-1)!! = 1`.
/// `DomainError` for `n < -1` or fractional `n`. `Overflow` on overflow.
pub fn fact_double(n: f64) -> MathResult<Number> {
    if is_na_real(n) {
        return Ok(Number::Float(na_real()));
    }
    if !n.is_finite() || n.trunc() != n {
        return Err(MathError::DomainError {
            function: "fact_double",
            what: format!("expected an integer value, got {n}"),
        });
    }
    let ni = n as i64;
    if ni < -1 {
        return Err(MathError::DomainError {
            function: "fact_double",
            what: format!("expected n >= -1, got {n}"),
        });
    }
    if ni <= 0 {
        return Ok(Number::Float(1.0));
    }
    let mut result = 1.0_f64;
    let mut k = ni;
    while k > 1 {
        result *= k as f64;
        if !result.is_finite() {
            return Err(MathError::Overflow {
                function: "fact_double",
            });
        }
        k -= 2;
    }
    Ok(Number::Float(result))
}

/// Excel `COMBIN(n, k)`. Number of `k`-element subsets of an `n`-element set.
/// Equals `n! / (k! * (n-k)!)`. Computed multiplicatively for numerical
/// stability.
///
/// `DomainError` if `n < 0`, `k < 0`, `k > n`, or either is fractional.
/// `Overflow` if the running product exceeds `f64::MAX`.
pub fn combin(n: f64, k: f64) -> MathResult<Number> {
    if is_na_real(n) || is_na_real(k) {
        return Ok(Number::Float(na_real()));
    }
    let ni = coerce_nonneg_int("combin", n)?;
    let ki = coerce_nonneg_int("combin", k)?;
    if ki > ni {
        return Err(MathError::DomainError {
            function: "combin",
            what: format!("k ({ki}) must not exceed n ({ni})"),
        });
    }
    // Use the smaller of k and n-k to minimize multiplications.
    let k_min = ki.min(ni - ki);
    let mut result = 1.0_f64;
    for i in 0..k_min {
        result *= (ni - i) as f64;
        result /= (i + 1) as f64;
        if !result.is_finite() {
            return Err(MathError::Overflow { function: "combin" });
        }
    }
    Ok(Number::Float(result.round()))
}

/// Excel `COMBINA(n, k)`. Combinations with repetition: `C(n+k-1, k)`.
pub fn combina(n: f64, k: f64) -> MathResult<Number> {
    if is_na_real(n) || is_na_real(k) {
        return Ok(Number::Float(na_real()));
    }
    let ni = coerce_nonneg_int("combina", n)?;
    let ki = coerce_nonneg_int("combina", k)?;
    if ni == 0 && ki == 0 {
        return Ok(Number::Float(1.0));
    }
    if ni == 0 {
        return Err(MathError::DomainError {
            function: "combina",
            what: "n must be >= 1 when k > 0".into(),
        });
    }
    combin((ni + ki - 1) as f64, ki as f64)
}

/// Excel `PERMUT(n, k)`. Number of ordered `k`-tuples drawn without
/// replacement from an `n`-element set. Equals `n! / (n - k)!`.
pub fn permut(n: f64, k: f64) -> MathResult<Number> {
    if is_na_real(n) || is_na_real(k) {
        return Ok(Number::Float(na_real()));
    }
    let ni = coerce_nonneg_int("permut", n)?;
    let ki = coerce_nonneg_int("permut", k)?;
    if ki > ni {
        return Err(MathError::DomainError {
            function: "permut",
            what: format!("k ({ki}) must not exceed n ({ni})"),
        });
    }
    let mut result = 1.0_f64;
    for i in 0..ki {
        result *= (ni - i) as f64;
        if !result.is_finite() {
            return Err(MathError::Overflow { function: "permut" });
        }
    }
    Ok(Number::Float(result))
}

/// Excel `PERMUTATIONA(n, k)`. Permutations with repetition: `n^k`.
pub fn permuta(n: f64, k: f64) -> MathResult<Number> {
    if is_na_real(n) || is_na_real(k) {
        return Ok(Number::Float(na_real()));
    }
    let ni = coerce_nonneg_int("permuta", n)?;
    let ki = coerce_nonneg_int("permuta", k)?;
    let result = (ni as f64).powi(ki as i32);
    if !result.is_finite() {
        return Err(MathError::Overflow {
            function: "permuta",
        });
    }
    Ok(Number::Float(result))
}

/// Excel `MULTINOMIAL(k1, k2, ..., kr)`. Computes
/// `(k1 + ... + kr)! / (k1! * k2! * ... * kr!)`. Stable: multiplies as it
/// expands the sum.
pub fn multinomial(args: &[f64]) -> MathResult<Number> {
    // Any NA in -> NA out.
    for &v in args {
        if is_na_real(v) {
            return Ok(Number::Float(na_real()));
        }
    }
    if args.is_empty() {
        return Ok(Number::Float(1.0));
    }
    // Validate and accumulate.
    let mut counts: Vec<u64> = Vec::with_capacity(args.len());
    for &v in args {
        counts.push(coerce_nonneg_int("multinomial", v)?);
    }

    // multinomial(k1, ..., kr) = C(s1+s2, s2) * C(s1+s2+s3, s3) * ...
    // where s_i is the running sum. Stable and avoids huge intermediate
    // factorials.
    let mut total: u64 = 0;
    let mut result = 1.0_f64;
    for k in counts {
        total = total.saturating_add(k);
        if k == 0 {
            continue;
        }
        match combin(total as f64, k as f64)? {
            Number::Float(v) => result *= v,
            _ => unreachable!("combin always returns Number::Float"),
        }
        if !result.is_finite() {
            return Err(MathError::Overflow {
                function: "multinomial",
            });
        }
    }
    Ok(Number::Float(result))
}

// --- helpers ---

fn coerce_nonneg_int(function: &'static str, value: f64) -> MathResult<u64> {
    if !value.is_finite() || value < 0.0 || value.trunc() != value {
        return Err(MathError::DomainError {
            function,
            what: format!("expected a non-negative integer, got {value}"),
        });
    }
    if value > i64::MAX as f64 {
        return Err(MathError::Overflow { function });
    }
    Ok(value as u64)
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
    fn fact_known_values() {
        assert_eq!(extract(fact(0.0).unwrap()), 1.0);
        assert_eq!(extract(fact(1.0).unwrap()), 1.0);
        assert_eq!(extract(fact(5.0).unwrap()), 120.0);
        assert_eq!(extract(fact(10.0).unwrap()), 3628800.0);
        // 170! fits, 171! does not.
        assert!(extract(fact(170.0).unwrap()).is_finite());
        assert!(matches!(
            fact(171.0).unwrap_err(),
            MathError::Overflow { .. }
        ));
        assert!(matches!(
            fact(-1.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
        assert!(matches!(
            fact(2.5).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn fact_double_known_values() {
        assert_eq!(extract(fact_double(0.0).unwrap()), 1.0);
        assert_eq!(extract(fact_double(-1.0).unwrap()), 1.0);
        assert_eq!(extract(fact_double(1.0).unwrap()), 1.0);
        assert_eq!(extract(fact_double(2.0).unwrap()), 2.0);
        assert_eq!(extract(fact_double(7.0).unwrap()), 105.0); // 7*5*3*1
        assert_eq!(extract(fact_double(8.0).unwrap()), 384.0); // 8*6*4*2
        assert!(matches!(
            fact_double(-2.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn combin_known_values() {
        assert_eq!(extract(combin(5.0, 2.0).unwrap()), 10.0);
        assert_eq!(extract(combin(5.0, 0.0).unwrap()), 1.0);
        assert_eq!(extract(combin(5.0, 5.0).unwrap()), 1.0);
        assert_eq!(extract(combin(10.0, 3.0).unwrap()), 120.0);
        // Symmetry: C(n,k) = C(n,n-k)
        assert_eq!(
            extract(combin(20.0, 7.0).unwrap()),
            extract(combin(20.0, 13.0).unwrap())
        );
        assert!(matches!(
            combin(3.0, 5.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn combina_known_values() {
        // C(n+k-1, k)
        assert_eq!(extract(combina(4.0, 3.0).unwrap()), 20.0); // C(6,3)
        assert_eq!(extract(combina(1.0, 5.0).unwrap()), 1.0); // C(5,5)
        assert_eq!(extract(combina(0.0, 0.0).unwrap()), 1.0);
    }

    #[test]
    fn permut_known_values() {
        assert_eq!(extract(permut(5.0, 2.0).unwrap()), 20.0);
        assert_eq!(extract(permut(10.0, 3.0).unwrap()), 720.0);
        assert_eq!(extract(permut(5.0, 0.0).unwrap()), 1.0);
        assert!(matches!(
            permut(3.0, 5.0).unwrap_err(),
            MathError::DomainError { .. }
        ));
    }

    #[test]
    fn permuta_known_values() {
        assert_eq!(extract(permuta(2.0, 3.0).unwrap()), 8.0);
        assert_eq!(extract(permuta(10.0, 2.0).unwrap()), 100.0);
        assert_eq!(extract(permuta(0.0, 0.0).unwrap()), 1.0);
    }

    #[test]
    fn multinomial_known_values() {
        // (2 + 3 + 4)! / (2! 3! 4!) = 1260
        assert_eq!(extract(multinomial(&[2.0, 3.0, 4.0]).unwrap()), 1260.0);
        assert_eq!(extract(multinomial(&[]).unwrap()), 1.0);
        assert_eq!(extract(multinomial(&[5.0]).unwrap()), 1.0); // 5!/5! = 1
        assert_eq!(extract(multinomial(&[1.0, 1.0]).unwrap()), 2.0);
    }

    #[test]
    fn combinatorics_na_propagates() {
        let na_ok = |n: Number| matches!(n, Number::Float(v) if is_na_real(v));
        assert!(na_ok(fact(na_real()).unwrap()));
        assert!(na_ok(fact_double(na_real()).unwrap()));
        assert!(na_ok(combin(na_real(), 2.0).unwrap()));
        assert!(na_ok(combin(5.0, na_real()).unwrap()));
        assert!(na_ok(permut(na_real(), 2.0).unwrap()));
        assert!(na_ok(combina(na_real(), 2.0).unwrap()));
        assert!(na_ok(permuta(2.0, na_real()).unwrap()));
        assert!(na_ok(multinomial(&[1.0, na_real(), 3.0]).unwrap()));
    }
}
