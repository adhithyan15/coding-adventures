//! Integer factorization.
//!
//! `factor_integer` produces a list of `(prime, exponent)` pairs for any
//! 64-bit integer.  `factorize_ir` wraps this in symbolic IR so the result
//! can be fed directly back into a CAS pipeline.
//!
//! # Algorithm
//!
//! Trial division: divide by 2 (handles all even factors), then by all odd
//! integers up to √|n|.  After the loop, if the remaining value `> 1` it is
//! a prime factor with exponent 1.
//!
//! Complexity: O(√|n|).  Practical for values up to ~10^12 (√n ≈ 10^6
//! iterations).
//!
//! # IR representation
//!
//! A factored integer is expressed as a product of prime powers:
//!
//! ```text
//! 12  →  2² × 3   →  Mul(Pow(2, 2), 3)
//! -12 →  -1 × 2² × 3  →  Mul(-1, Pow(2, 2), 3)
//! 7   →  7   (prime, unchanged)
//! 1   →  1   (unit, unchanged)
//! 0   →  0   (zero, unchanged)
//! ```

use symbolic_ir::{apply, int, sym, IRNode, MUL, POW};

// ---------------------------------------------------------------------------
// Integer factorization
// ---------------------------------------------------------------------------

/// Prime factorization of `n`.
///
/// Returns a sorted list of `(prime, exponent)` pairs such that:
/// `|n| = ∏ prime^exponent`.
///
/// - Returns an empty vector for `|n| ≤ 1` (0 and ±1 have no prime factors).
/// - Handles negative inputs: the sign is ignored (factor `|n|`).
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::factor_integer;
///
/// assert_eq!(factor_integer(12), vec![(2, 2), (3, 1)]);
/// assert_eq!(factor_integer(-12), vec![(2, 2), (3, 1)]);  // sign ignored
/// assert_eq!(factor_integer(7), vec![(7, 1)]);   // prime
/// assert_eq!(factor_integer(1), vec![]);
/// assert_eq!(factor_integer(0), vec![]);
/// assert_eq!(factor_integer(360), vec![(2, 3), (3, 2), (5, 1)]);
/// ```
pub fn factor_integer(n: i64) -> Vec<(i64, u32)> {
    let mut n = n.abs();
    if n <= 1 {
        return vec![];
    }

    let mut factors: Vec<(i64, u32)> = Vec::new();

    // Divide out the factor 2 first (handles all even inputs cheaply).
    if n % 2 == 0 {
        let mut exp = 0u32;
        while n % 2 == 0 {
            n /= 2;
            exp += 1;
        }
        factors.push((2, exp));
    }

    // Trial-divide by odd numbers 3, 5, 7, … up to √n.
    let mut d = 3i64;
    while d * d <= n {
        if n % d == 0 {
            let mut exp = 0u32;
            while n % d == 0 {
                n /= d;
                exp += 1;
            }
            factors.push((d, exp));
        }
        d += 2;
    }

    // Remaining value > 1 is a prime factor with exponent 1.
    if n > 1 {
        factors.push((n, 1));
    }

    factors
}

// ---------------------------------------------------------------------------
// IR representation
// ---------------------------------------------------------------------------

/// Express `expr` (which must be an `IRNode::Integer`) as a product of prime
/// powers in symbolic IR.
///
/// | Input | Output |
/// |-------|--------|
/// | `0`   | `0` (unchanged) |
/// | `1`   | `1` (unchanged) |
/// | `-1`  | `-1` (unchanged) |
/// | `-6`  | `Mul(-1, 2, 3)` |
/// | `12`  | `Mul(Pow(2, 2), 3)` |
/// | `7`   | `7` (prime — unchanged) |
///
/// Non-`Integer` nodes are returned unchanged.
///
/// # Examples
///
/// ```rust
/// use cas_number_theory::factorize_ir;
/// use symbolic_ir::{apply, int, sym, IRNode, MUL, POW};
///
/// // 12 = 2² × 3
/// let result = factorize_ir(&int(12));
/// if let IRNode::Apply(a) = &result {
///     assert_eq!(a.head, sym(MUL));
///     assert_eq!(a.args.len(), 2);
/// }
///
/// // primes are returned unchanged
/// assert_eq!(factorize_ir(&int(7)), int(7));
/// assert_eq!(factorize_ir(&int(1)), int(1));
/// assert_eq!(factorize_ir(&int(0)), int(0));
/// ```
pub fn factorize_ir(expr: &IRNode) -> IRNode {
    let n = match expr {
        IRNode::Integer(n) => *n,
        _ => return expr.clone(),
    };

    if n == 0 || n == 1 || n == -1 {
        return expr.clone();
    }

    let sign = if n < 0 { -1i64 } else { 1 };
    let factors = factor_integer(n);

    // An empty factor list means |n| ≤ 1 — shouldn't happen here after the
    // check above, but defend against it.
    if factors.is_empty() {
        return expr.clone();
    }

    // If n is prime, just return it unchanged.
    if factors.len() == 1 && factors[0].1 == 1 && sign == 1 {
        return expr.clone();
    }

    // Build the list of terms for the Mul(…) node.
    let mut terms: Vec<IRNode> = Vec::new();

    // Prepend −1 for negative inputs.
    if sign == -1 {
        terms.push(int(-1));
    }

    for (p, e) in factors {
        if e == 1 {
            terms.push(int(p));
        } else {
            terms.push(apply(sym(POW), vec![int(p), int(e as i64)]));
        }
    }

    // A single term (e.g. negative prime like −7) — wrap in Mul anyway to
    // keep the negative-sign prefix.
    if terms.len() == 1 {
        // This can happen for e.g. -7 → [int(-1), int(7)] … wait, that's 2.
        // Actually len==1 only when sign==1 and there's one prime factor.
        // We already short-circuit that case above.  But be safe:
        return terms.into_iter().next().unwrap();
    }

    apply(sym(MUL), terms)
}
