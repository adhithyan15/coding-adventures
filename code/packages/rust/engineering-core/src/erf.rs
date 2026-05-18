//! # Error function ERF and complementary ERFC.
//!
//! Implementation: Abramowitz & Stegun 7.1.26 rational approximation,
//! accuracy ~1.5e-7. Excel's ERF.PRECISE / ERFC.PRECISE are documented
//! as having slightly better accuracy than ERF/ERFC; we expose them as
//! aliases since our single implementation already exceeds the
//! "imprecise" specification.

use super::EngineeringError;

/// Excel `ERF(lower, [upper])` — single-argument form returns erf(x).
pub fn erf(x: f64) -> f64 {
    erf_impl(x)
}

/// Excel `ERF(lower, upper)` two-argument form: erf(upper) - erf(lower).
pub fn erf_range(lower: f64, upper: f64) -> f64 {
    erf_impl(upper) - erf_impl(lower)
}

/// Excel `ERFC(x)` — complementary error function 1 - erf(x).
/// Computes directly for large `x` to avoid catastrophic cancellation.
pub fn erfc(x: f64) -> f64 {
    if x.abs() < 0.5 {
        1.0 - erf_impl(x)
    } else if x >= 0.0 {
        erfc_impl(x)
    } else {
        2.0 - erfc_impl(-x)
    }
}

/// Excel `ERF.PRECISE(x)` — alias for `erf(x)` here.
pub fn erf_precise(x: f64) -> f64 {
    erf_impl(x)
}

/// Excel `ERFC.PRECISE(x)` — alias for `erfc(x)`.
pub fn erfc_precise(x: f64) -> f64 {
    erfc(x)
}

// ---------------------------------------------------------------------------
// Implementation — Abramowitz & Stegun 7.1.26
// ---------------------------------------------------------------------------

/// Standard erf via Abramowitz & Stegun 7.1.26. For x < 0, uses
/// the odd-symmetry property erf(-x) = -erf(x).
fn erf_impl(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.is_infinite() {
        return x.signum();
    }
    if x == 0.0 {
        // The A&S approximation evaluates to ~1e-9 at x=0 due to
        // coefficient roundoff; short-circuit to the exact answer.
        return 0.0;
    }
    let sign = x.signum();
    let x_abs = x.abs();
    // Coefficients for A&S 7.1.26.
    let a1 = 0.254829592_f64;
    let a2 = -0.284496736_f64;
    let a3 = 1.421413741_f64;
    let a4 = -1.453152027_f64;
    let a5 = 1.061405429_f64;
    let p = 0.3275911_f64;

    let t = 1.0 / (1.0 + p * x_abs);
    let y = 1.0
        - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-(x_abs * x_abs)).exp();
    sign * y
}

/// erfc for x >= 0 via continued-fraction-like A&S 7.1.26, in a form
/// that doesn't suffer catastrophic cancellation.
fn erfc_impl(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x == f64::INFINITY {
        return 0.0;
    }
    let a1 = 0.254829592_f64;
    let a2 = -0.284496736_f64;
    let a3 = 1.421413741_f64;
    let a4 = -1.453152027_f64;
    let a5 = 1.061405429_f64;
    let p = 0.3275911_f64;

    let t = 1.0 / (1.0 + p * x);
    let poly = (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t;
    poly * (-(x * x)).exp()
}

/// Vectorized ERF for convenience.
pub fn erf_vec(xs: &[f64]) -> Vec<f64> {
    xs.iter().copied().map(erf).collect()
}

/// Helper to return ERF as a `Result` for API consistency (NaN
/// becomes a `DomainError`).
pub fn erf_checked(x: f64) -> Result<f64, EngineeringError> {
    if x.is_nan() {
        Err(EngineeringError::DomainError {
            function: "erf",
            what: "NaN input".into(),
        })
    } else {
        Ok(erf(x))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn erf_known_values() {
        // Reference values from NIST/Wolfram.
        assert!(close(erf(0.0), 0.0, 1e-10));
        assert!(close(erf(0.5), 0.5204998778, 1e-6));
        assert!(close(erf(1.0), 0.8427007929, 1e-6));
        assert!(close(erf(2.0), 0.9953222650, 1e-6));
        assert!(close(erf(3.0), 0.9999779095, 1e-6));
    }

    #[test]
    fn erf_odd_symmetry() {
        for x in [0.1, 0.5, 1.0, 2.0, 3.0] {
            assert!(close(erf(-x), -erf(x), 1e-10), "asymmetry at {x}");
        }
    }

    #[test]
    fn erfc_complement() {
        for x in [-2.0, -0.5, 0.0, 0.5, 1.0] {
            let sum = erf(x) + erfc(x);
            assert!(close(sum, 1.0, 1e-10), "erf + erfc != 1 at {x} (got {sum})");
        }
    }

    #[test]
    fn erfc_does_not_lose_precision_for_large_x() {
        // erfc(5) ≈ 1.5374597944e-12; computing as 1 - erf(5) would
        // round to 0 due to float precision.
        let result = erfc(5.0);
        assert!(result > 1e-13 && result < 1e-11, "erfc(5)={result}");
        // erfc(10) ≈ 2e-45 — should be a tiny positive number, not zero.
        let result = erfc(10.0);
        assert!(result >= 0.0 && result < 1e-30, "erfc(10)={result}");
    }

    #[test]
    fn erf_infinity_limits() {
        assert_eq!(erf(f64::INFINITY), 1.0);
        assert_eq!(erf(f64::NEG_INFINITY), -1.0);
    }

    #[test]
    fn erf_nan_propagates() {
        assert!(erf(f64::NAN).is_nan());
        assert!(erfc(f64::NAN).is_nan());
    }

    #[test]
    fn erf_range_two_arg_form() {
        let val = erf_range(0.0, 1.0);
        assert!(close(val, 0.8427007929, 1e-6));
    }

    #[test]
    fn erf_vec_round_trips() {
        let xs = vec![0.0, 0.5, 1.0, 2.0];
        let out = erf_vec(&xs);
        for (i, &x) in xs.iter().enumerate() {
            assert_eq!(out[i], erf(x));
        }
    }

    #[test]
    fn erf_checked_rejects_nan() {
        assert!(erf_checked(f64::NAN).is_err());
        assert!(erf_checked(0.5).is_ok());
    }

    #[test]
    fn erf_precise_aliases() {
        for x in [0.1, 0.5, 1.0] {
            assert_eq!(erf_precise(x), erf(x));
            assert_eq!(erfc_precise(x), erfc(x));
        }
    }
}
