//! Exact values of sin, cos, and tan at rational multiples of π.
//!
//! For an argument `n/d · π`, each trig function returns an exact algebraic
//! `IRNode` when the denominator (after reduction modulo 2) is in
//! `{1, 2, 3, 4, 6}`.
//!
//! # Unit-circle table
//!
//! | angle (fraction of π) | degrees | sin       | cos       | tan       |
//! |-----------------------|---------|-----------|-----------|-----------|
//! | 0                     | 0°      | 0         | 1         | 0         |
//! | 1/6                   | 30°     | 1/2       | √3/2      | 1/√3      |
//! | 1/4                   | 45°     | √2/2      | √2/2      | 1         |
//! | 1/3                   | 60°     | √3/2      | 1/2       | √3        |
//! | 1/2                   | 90°     | 1         | 0         | ±∞        |
//! | 2/3                   | 120°    | √3/2      | −1/2      | −√3       |
//! | 3/4                   | 135°    | √2/2      | −√2/2     | −1        |
//! | 5/6                   | 150°    | 1/2       | −√3/2     | −1/√3     |
//! | 1                     | 180°    | 0         | −1        | 0         |
//! | (and so on for [π, 2π) by symmetry) |  |           |           |         |
//!
//! Values like `√2/2` are represented exactly in IR as
//! `Mul(Rational(1,2), Sqrt(2))`.  Undefined cases (tan at π/2, 3π/2)
//! return `None`.
//!
//! # Input reduction
//!
//! All three functions normalise the input fraction `num/den` to the
//! canonical range `[0, 2)` using Euclidean remainder, so callers may
//! pass any rational multiple of π (including negatives and values > 2π).

use symbolic_ir::{apply, int, rat, sym, IRNode, MUL, NEG, SQRT};

use crate::constants::PI_SYMBOL;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Exact value of `sin(num/den · π)`.
///
/// Returns `None` for denominators not in `{1, 2, 3, 4, 6}` after reduction.
///
/// # Examples
///
/// ```rust
/// use cas_trig::special::{sin_at_pi_multiple, cos_at_pi_multiple};
/// use symbolic_ir::{int, rat};
///
/// // sin(π/2) = 1
/// assert_eq!(sin_at_pi_multiple(1, 2), Some(int(1)));
///
/// // sin(π/6) = 1/2
/// assert_eq!(sin_at_pi_multiple(1, 6), Some(rat(1, 2)));
///
/// // sin(2π) = sin(0) = 0  (reduction modulo 2π)
/// assert_eq!(sin_at_pi_multiple(2, 1), Some(int(0)));
/// ```
pub fn sin_at_pi_multiple(num: i64, den: i64) -> Option<IRNode> {
    let (k, m) = reduce_pi_fraction(num, den);
    match (k, m) {
        // 0° and 360°
        (0, _) => Some(int(0)),
        // 30° and 150°  — sin = 1/2
        (1, 6) | (5, 6) => Some(rat(1, 2)),
        // 210° and 330° — sin = -1/2
        (7, 6) | (11, 6) => Some(rat(-1, 2)),
        // 45° and 135° — sin = √2/2
        (1, 4) | (3, 4) => Some(sqrt2_over_2()),
        // 225° and 315° — sin = -√2/2
        (5, 4) | (7, 4) => Some(neg_surd(sqrt2_over_2())),
        // 60° and 120° — sin = √3/2
        (1, 3) | (2, 3) => Some(sqrt3_over_2()),
        // 240° and 300° — sin = -√3/2
        (4, 3) | (5, 3) => Some(neg_surd(sqrt3_over_2())),
        // 90° — sin = 1
        (1, 2) => Some(int(1)),
        // 270° — sin = -1
        (3, 2) => Some(int(-1)),
        // 180° — sin = 0
        (1, 1) => Some(int(0)),
        _ => None,
    }
}

/// Exact value of `cos(num/den · π)`.
///
/// Returns `None` for denominators not in `{1, 2, 3, 4, 6}` after reduction.
///
/// # Examples
///
/// ```rust
/// use cas_trig::special::cos_at_pi_multiple;
/// use symbolic_ir::{int, rat};
///
/// // cos(π) = -1
/// assert_eq!(cos_at_pi_multiple(1, 1), Some(int(-1)));
///
/// // cos(π/3) = 1/2
/// assert_eq!(cos_at_pi_multiple(1, 3), Some(rat(1, 2)));
/// ```
pub fn cos_at_pi_multiple(num: i64, den: i64) -> Option<IRNode> {
    let (k, m) = reduce_pi_fraction(num, den);
    match (k, m) {
        // 0° — cos = 1
        (0, _) => Some(int(1)),
        // 30° and 330° — cos = √3/2
        (1, 6) | (11, 6) => Some(sqrt3_over_2()),
        // 150° and 210° — cos = -√3/2
        (5, 6) | (7, 6) => Some(neg_surd(sqrt3_over_2())),
        // 45° and 315° — cos = √2/2
        (1, 4) | (7, 4) => Some(sqrt2_over_2()),
        // 135° and 225° — cos = -√2/2
        (3, 4) | (5, 4) => Some(neg_surd(sqrt2_over_2())),
        // 60° and 300° — cos = 1/2
        (1, 3) | (5, 3) => Some(rat(1, 2)),
        // 120° and 240° — cos = -1/2
        (2, 3) | (4, 3) => Some(rat(-1, 2)),
        // 90° and 270° — cos = 0
        (1, 2) | (3, 2) => Some(int(0)),
        // 180° — cos = -1
        (1, 1) => Some(int(-1)),
        _ => None,
    }
}

/// Exact value of `tan(num/den · π)`.
///
/// Returns `None` for undefined angles (π/2, 3π/2) and for denominators
/// not in `{1, 2, 3, 4, 6}` after reduction.
///
/// # Examples
///
/// ```rust
/// use cas_trig::special::tan_at_pi_multiple;
/// use symbolic_ir::int;
///
/// // tan(π/4) = 1
/// assert_eq!(tan_at_pi_multiple(1, 4), Some(int(1)));
///
/// // tan(π/2) = undefined
/// assert_eq!(tan_at_pi_multiple(1, 2), None);
/// ```
pub fn tan_at_pi_multiple(num: i64, den: i64) -> Option<IRNode> {
    let (k, m) = reduce_pi_fraction(num, den);
    match (k, m) {
        // 0° and 180° — tan = 0
        (0, _) | (1, 1) => Some(int(0)),
        // 30° and 210° — tan = 1/√3 = √3/3
        (1, 6) | (7, 6) => Some(inv_sqrt3()),
        // 150° and 330° — tan = -1/√3
        (5, 6) | (11, 6) => Some(neg_surd(inv_sqrt3())),
        // 45° and 225° — tan = 1
        (1, 4) | (5, 4) => Some(int(1)),
        // 135° and 315° — tan = -1
        (3, 4) | (7, 4) => Some(int(-1)),
        // 60° and 240° — tan = √3
        (1, 3) | (4, 3) => Some(sqrt3()),
        // 120° and 300° — tan = -√3
        (2, 3) | (5, 3) => Some(neg_surd(sqrt3())),
        // 90° and 270° — undefined
        (1, 2) | (3, 2) => None,
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Reduce `num/den · π` to a canonical fraction `(k, m)` in `[0, 2)`,
/// in lowest terms with `m > 0`.
///
/// Algorithm (using Euclidean remainder, which always gives a non-negative
/// result in Rust):
///
/// ```text
/// 1. Normalise sign: if den < 0, flip both num and den.
/// 2. k = num rem_euclid (2 · den)          ∈ [0, 2·den)
/// 3. g = gcd(k, den)
/// 4. return (k / g, den / g)
/// ```
///
/// This ensures that, e.g., `(−1, 2)` becomes `(3, 2)` (= 3π/2 = −π/2 mod 2π),
/// and `(2, 1)` becomes `(0, 1)` (= 2π = 0 mod 2π).
fn reduce_pi_fraction(num: i64, den: i64) -> (i64, i64) {
    debug_assert!(den != 0, "denominator must not be zero");
    let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
    // Euclidean remainder gives a non-negative result in [0, 2·den).
    let k = num.rem_euclid(2 * den);
    let g = gcd_u64(k.unsigned_abs(), den as u64) as i64;
    // g ≥ 1 because den > 0.
    (k / g, den / g)
}

/// Unsigned GCD via Euclidean algorithm.
fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Negate a surd-valued node: wraps in `Neg(…)`.
///
/// Handles `Integer` values directly (avoids `Neg(Integer(1))`).
fn neg_surd(n: IRNode) -> IRNode {
    match n {
        IRNode::Integer(v) => int(-v),
        IRNode::Rational(num, den) => rat(-num, den),
        other => apply(sym(NEG), vec![other]),
    }
}

/// Build `√2 / 2 = Mul(Rational(1,2), Sqrt(2))`.
///
/// This is the exact representation of `sin(π/4) = cos(π/4)`.
fn sqrt2_over_2() -> IRNode {
    apply(sym(MUL), vec![rat(1, 2), apply(sym(SQRT), vec![int(2)])])
}

/// Build `√3 / 2 = Mul(Rational(1,2), Sqrt(3))`.
///
/// This is the exact representation of `sin(π/3) = cos(π/6)`.
fn sqrt3_over_2() -> IRNode {
    apply(sym(MUL), vec![rat(1, 2), apply(sym(SQRT), vec![int(3)])])
}

/// Build `√3 = Sqrt(3)`.
///
/// This is the exact representation of `tan(π/3)`.
fn sqrt3() -> IRNode {
    apply(sym(SQRT), vec![int(3)])
}

/// Build `1/√3 = √3/3 = Mul(Rational(1,3), Sqrt(3))`.
///
/// Derivation: `1/√3 = 1/√3 · √3/√3 = √3/3 = (1/3)·√3`.
///
/// This is the exact representation of `tan(π/6)`.
fn inv_sqrt3() -> IRNode {
    apply(sym(MUL), vec![rat(1, 3), apply(sym(SQRT), vec![int(3)])])
}

// Make PI_SYMBOL reachable from the module (suppress unused warning on import)
#[allow(dead_code)]
const _PI: &str = PI_SYMBOL;
