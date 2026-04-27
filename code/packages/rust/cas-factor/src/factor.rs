//! Top-level `factor_integer_polynomial` orchestrator.
//!
//! Given an integer polynomial as a coefficient list, returns:
//!
//! 1. The integer **content** — the GCD of every coefficient (always positive).
//! 2. A list of `(factor_coeffs, multiplicity)` pairs.  Phase 1 only finds
//!    *linear* factors via the rational-root test; any irreducible residual is
//!    appended with multiplicity 1.
//!
//! # Examples
//!
//! ```
//! use cas_factor::factor_integer_polynomial;
//!
//! // x^2 - 1 = 1 * (x - 1)(x + 1)
//! let (c, factors) = factor_integer_polynomial(&[-1, 0, 1]);
//! assert_eq!(c, 1);
//! // factors contain ([-1, 1], 1) and ([1, 1], 1) in some order
//! assert_eq!(factors.len(), 2);
//!
//! // 2x^2 + 4x + 2 = 2 * (x + 1)^2
//! let (c, factors) = factor_integer_polynomial(&[2, 4, 2]);
//! assert_eq!(c, 2);
//! assert_eq!(factors, vec![(vec![1, 1], 2)]);
//!
//! // x^2 + 1 — irreducible over Q
//! let (c, factors) = factor_integer_polynomial(&[1, 0, 1]);
//! assert_eq!(c, 1);
//! assert_eq!(factors, vec![(vec![1, 0, 1], 1)]);
//! ```

use crate::polynomial::{content, normalize, primitive_part};
use crate::rational_roots::extract_linear_factors;

/// A factored polynomial: `Vec<(coeffs, multiplicity)>`.
pub type FactorList = Vec<(Vec<i64>, usize)>;

/// Factor a univariate integer polynomial over ℤ (linear factors only).
///
/// # Returns
///
/// `(content, factors)` where the product `content * ∏ factor(x)^mult`
/// equals `p` (up to the sign convention: content is always positive and any
/// leading `-1` residual is absorbed into the content).
///
/// The `factors` list contains:
/// - One entry `([-root, 1], mult)` per integer root found.
/// - Optionally one entry for the irreducible residual (if it isn't `[1]` or
///   `[-1]`).
pub fn factor_integer_polynomial(p: &[i64]) -> (i64, FactorList) {
    if p.is_empty() {
        return (0, vec![]);
    }

    let mut c = content(p);
    let pp = primitive_part(p);
    let (linear_factors, residual) = extract_linear_factors(&pp);

    let mut factors: FactorList = Vec::new();

    // Translate each `(root, mult)` into a coefficient list `[-root, 1]`.
    // A root `r` means the linear factor is `(x - r)`, i.e., coefficients
    // `[-r, 1]` (constant term first).
    for (root, mult) in &linear_factors {
        factors.push((vec![-root, 1], *mult));
    }

    // Handle the residual.
    let residual_norm = normalize(&residual);
    if residual_norm == vec![-1i64] {
        // The trailing -1 is absorbed into the content sign.
        c = -c;
    } else if !residual_norm.is_empty() && residual_norm != vec![1i64] {
        // Non-trivial irreducible residual.
        factors.push((residual_norm, 1));
    }

    (c, factors)
}
