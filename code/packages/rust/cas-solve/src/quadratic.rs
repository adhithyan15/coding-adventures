//! Quadratic-equation closed form via the quadratic formula.
//!
//! For `a·x² + b·x + c = 0` with rational coefficients:
//!
//! ```text
//! x = (−b ± √disc) / (2a)     disc = b² − 4ac
//! ```
//!
//! Three cases:
//!
//! 1. **`disc` is a perfect square** — both roots are rational.
//! 2. **`disc > 0` but not a perfect square** — roots are real-irrational,
//!    expressed as `(-b ± Sqrt(disc)) / (2a)` in IR.
//! 3. **`disc < 0`** — roots are complex, expressed as `r ± k·%i` in IR.
//!
//! When `a == 0`, falls back to [`crate::linear::solve_linear`].

use symbolic_ir::{apply, sym, IRNode, ADD, DIV, MUL, SQRT, SUB};

use crate::frac::Frac;
use crate::linear::solve_linear;
use crate::SolveResult;

/// The imaginary unit `%i` (Maxima/MACSYMA convention).
pub const I_UNIT: &str = "%i";

/// Solve `a·x² + b·x + c = 0` over ℚ (with `%i` if disc < 0).
///
/// # Returns
///
/// A [`SolveResult`] containing zero, one, or two `IRNode` roots, or
/// `SolveResult::All` if `a = b = c = 0`.
///
/// # Examples
///
/// ```rust
/// use cas_solve::{solve_quadratic, SolveResult};
/// use cas_solve::frac::Frac;
/// use symbolic_ir::int;
///
/// // x^2 - 5x + 6 = 0  →  {2, 3}
/// let r = solve_quadratic(
///     Frac::from_int(1), Frac::from_int(-5), Frac::from_int(6),
/// );
/// assert_eq!(r, SolveResult::Solutions(vec![int(2), int(3)]));
///
/// // x^2 - 4x + 4 = 0  →  repeated root 2
/// let r2 = solve_quadratic(
///     Frac::from_int(1), Frac::from_int(-4), Frac::from_int(4),
/// );
/// assert_eq!(r2, SolveResult::Solutions(vec![int(2)]));
/// ```
pub fn solve_quadratic(a: Frac, b: Frac, c: Frac) -> SolveResult {
    if a.is_zero() {
        return solve_linear(b, c);
    }

    // discriminant = b^2 - 4*a*c
    let four = Frac::from_int(4);
    let discriminant = b * b - four * a * c;
    let two_a = Frac::from_int(2) * a;
    let neg_b = -b;

    if discriminant > Frac::zero() {
        // Positive discriminant — two distinct real roots.
        match sqrt_or_rational(discriminant) {
            SqrtResult::Rational(sq) => {
                // Perfect square: rational roots.
                let mut roots = vec![
                    ((neg_b + sq) / two_a).to_irnode(),
                    ((neg_b - sq) / two_a).to_irnode(),
                ];
                roots.sort_by_key(irnode_sort_key);
                SolveResult::Solutions(roots)
            }
            SqrtResult::Irrational(sqrt_node) => {
                // Irrational: express as (-b ± Sqrt(disc)) / (2a).
                SolveResult::Solutions(vec![
                    build_irrational_root(neg_b, two_a, sqrt_node.clone(), 1),
                    build_irrational_root(neg_b, two_a, sqrt_node, -1),
                ])
            }
        }
    } else if discriminant == Frac::zero() {
        // Discriminant zero — single repeated root.
        let root = (neg_b / two_a).to_irnode();
        SolveResult::Solutions(vec![root])
    } else {
        // Negative discriminant — complex roots.
        let abs_disc = -discriminant;
        let sqrt_abs = sqrt_or_rational(abs_disc);
        SolveResult::Solutions(vec![
            build_complex_root(neg_b, two_a, sqrt_abs.clone(), 1),
            build_complex_root(neg_b, two_a, sqrt_abs, -1),
        ])
    }
}

// ---------------------------------------------------------------------------
// Square root helper
// ---------------------------------------------------------------------------

/// The result of trying to take the square root of a fraction.
#[derive(Clone)]
pub(crate) enum SqrtResult {
    /// The value was a perfect square — exact rational result.
    Rational(Frac),
    /// Not a perfect square — an IR `Sqrt(literal)` node.
    Irrational(IRNode),
}

/// If `value` is a positive perfect square of a fraction, return the rational
/// square root.  Otherwise return `Sqrt(value)` as an IR node.
fn sqrt_or_rational(value: Frac) -> SqrtResult {
    // Both numerator and denominator must be perfect squares.
    if value.numer >= 0 {
        if let (Some(rn), Some(rd)) = (isqrt(value.numer as u64), isqrt(value.denom as u64)) {
            return SqrtResult::Rational(Frac::new(rn as i64, rd as i64));
        }
    }
    SqrtResult::Irrational(apply(sym(SQRT), vec![value.to_irnode()]))
}

/// Integer square root if `n` is a perfect square; `None` otherwise.
fn isqrt(n: u64) -> Option<u64> {
    if n == 0 {
        return Some(0);
    }
    let r = (n as f64).sqrt() as u64;
    // Bracket-search to handle float-rounding edge cases for large n.
    for cand in r.saturating_sub(1)..=r + 1 {
        if cand * cand == n {
            return Some(cand);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// IR builders
// ---------------------------------------------------------------------------

/// Build `(-b ± Sqrt(disc)) / (2a)` for an irrational discriminant.
fn build_irrational_root(neg_b: Frac, two_a: Frac, sqrt_node: IRNode, sign: i32) -> IRNode {
    let head_op = if sign > 0 { ADD } else { SUB };
    let numer = apply(sym(head_op), vec![neg_b.to_irnode(), sqrt_node]);
    apply(sym(DIV), vec![numer, two_a.to_irnode()])
}

/// Build `-b/(2a) ± (Sqrt(|disc|)/(2a))·%i` for complex roots.
fn build_complex_root(neg_b: Frac, two_a: Frac, sqrt_abs: SqrtResult, sign: i32) -> IRNode {
    let real_part = (neg_b / two_a).to_irnode();
    let imag_part = match sqrt_abs {
        SqrtResult::Rational(sq) => {
            let coef = (sq / two_a).to_irnode();
            apply(sym(MUL), vec![coef, sym(I_UNIT)])
        }
        SqrtResult::Irrational(sqrt_node) => {
            let coef_ir = apply(sym(DIV), vec![sqrt_node, two_a.to_irnode()]);
            apply(sym(MUL), vec![coef_ir, sym(I_UNIT)])
        }
    };
    let head_op = if sign > 0 { ADD } else { SUB };
    apply(sym(head_op), vec![real_part, imag_part])
}

// ---------------------------------------------------------------------------
// Sort key for rational roots (so the test output is deterministic)
// ---------------------------------------------------------------------------

fn irnode_sort_key(node: &IRNode) -> (i64, i64) {
    match node {
        IRNode::Integer(n) => (*n, 1),
        IRNode::Rational(n, d) => (*n, *d),
        _ => (i64::MAX, 1),
    }
}
