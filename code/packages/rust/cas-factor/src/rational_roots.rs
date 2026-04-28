//! Rational-root test for linear factors over ℤ.
//!
//! ## Rational Root Theorem
//!
//! For an integer polynomial `a_0 + a_1·x + … + a_n·x^n`, any rational root
//! `p/q` (in lowest terms) satisfies:
//!
//! - `p | a_0`  (p divides the constant term)
//! - `q | a_n`  (q divides the leading coefficient)
//!
//! In Phase 1 we only chase **integer** roots (`q = 1`), which means we only
//! need the divisors of `a_0`.  This covers all textbook factoring cases and
//! is a sufficient subset for the CAS use case.
//!
//! ## Example
//!
//! ```text
//! x^2 - 1  candidates: ±1   roots: {-1, 1}
//! x^3 - 6x^2 + 11x - 6  candidates: ±1, ±2, ±3, ±6   roots: {1, 2, 3}
//! x^2 + 1  candidates: ±1   no roots
//! ```

use crate::polynomial::{divide_linear, divisors, evaluate, normalize, Poly};

/// Return all integer roots of `p`, each appearing once in the list.
///
/// Repeated roots are discovered through repeated division in
/// [`extract_linear_factors`].
///
/// ```
/// use cas_factor::rational_roots::find_integer_roots;
/// let mut roots = find_integer_roots(&[-1, 0, 1]);
/// roots.sort_unstable();
/// assert_eq!(roots, vec![-1i64, 1]);
/// ```
pub fn find_integer_roots(p: &[i64]) -> Vec<i64> {
    let p = normalize(p);
    if p.is_empty() {
        return vec![];
    }
    let constant = p[0];
    if constant == 0 {
        // x is a factor — 0 is a root.  Strip the leading factor of x and
        // recurse to find any remaining roots.
        let rest = &p[1..];
        let mut roots = vec![0i64];
        if !rest.is_empty() {
            roots.extend(find_integer_roots(rest));
        }
        return roots;
    }
    // Candidate set: ±divisors(constant_term).
    let pos_divs = divisors(constant);
    let mut candidates: Vec<i64> = pos_divs
        .iter()
        .flat_map(|&d| [d, -d])
        .collect();
    candidates.sort_unstable();
    candidates.dedup();

    candidates
        .into_iter()
        .filter(|&c| evaluate(&p, c) == 0)
        .collect()
}

/// Extract every integer-root linear factor from `p`, tracking multiplicities.
///
/// Returns `(factors, residual)` where:
///
/// - `factors` is a `Vec<(root, multiplicity)>` sorted ascending by root.
/// - `residual` is the polynomial remaining after all linear factors are
///   divided out.  This may be `[1]`, the empty list, or a higher-degree
///   irreducible polynomial.
///
/// The algorithm is a fixed-point loop: after each full pass over the current
/// roots, any roots with multiplicity > 1 will reappear in the next pass
/// because they survive one division.
///
/// # Example
///
/// ```text
/// x^2 - 1 = (x - 1)(x + 1)
/// factors = [(-1, 1), (1, 1)],  residual = [1]
///
/// x^2 + 2x + 1 = (x + 1)^2
/// factors = [(-1, 2)],  residual = [1]
///
/// x^2 + 1   (irreducible over Q)
/// factors = [],  residual = [1, 0, 1]
/// ```
pub fn extract_linear_factors(p: &[i64]) -> (Vec<(i64, usize)>, Poly) {
    let mut p = normalize(p);
    // BTreeMap gives sorted-key iteration for free.
    let mut factors: std::collections::BTreeMap<i64, usize> = std::collections::BTreeMap::new();
    loop {
        let roots = find_integer_roots(&p);
        if roots.is_empty() {
            break;
        }
        for r in &roots {
            p = divide_linear(&p, *r);
            *factors.entry(*r).or_insert(0) += 1;
        }
    }
    let factor_list: Vec<(i64, usize)> = factors.into_iter().collect();
    (factor_list, p)
}
