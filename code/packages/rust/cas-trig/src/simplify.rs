//! Main simplification entry points for trigonometric expressions.
//!
//! ## Evaluation strategy (three tiers)
//!
//! Each `*_eval` function receives the **argument** of the trig function (not
//! the whole `Sin(arg)` expression) and returns a simplified result:
//!
//! 1. **Special value**: if the argument is a recognised rational multiple of
//!    π (denominator 1, 2, 3, 4, or 6 after reduction), return an exact
//!    algebraic `Integer`, `Rational`, or `Sqrt(…)` node.
//! 2. **Numeric**: if the argument is a pure number (`Integer`, `Float`,
//!    `Rational`) or the constant `Pi`, compute the value as `f64` and snap
//!    near-integers back to exact form.
//! 3. **Unevaluated**: return `Sin(arg)` / `Cos(arg)` / `Tan(arg)`.
//!
//! ## Tree walker
//!
//! `trig_simplify` applies these rules everywhere inside an expression tree,
//! recursively simplifying arguments before simplifying each trig node.
//!
//! ## π-multiple recognition
//!
//! The helper `extract_pi_multiple` recognises these patterns as
//! `(num, den)` where the argument equals `(num/den) · π`:
//!
//! ```text
//! Integer(0)                 → (0, 1)
//! Symbol("Pi")               → (1, 1)
//! Neg(Symbol("Pi"))          → (-1, 1)
//! Mul(r, Pi) or Mul(Pi, r)   → (r.num, r.den)    for rational r
//! Neg(Mul(r, Pi))            → negated form
//! ```

use symbolic_ir::{apply, sym, IRNode, ACOS, ASIN, ATAN, COS, SIN, TAN};

use crate::constants::PI_SYMBOL;
use crate::numeric::{acos_numeric, asin_numeric, atan_numeric, cos_numeric, sin_numeric,
                     tan_numeric, to_float};
use crate::special::{cos_at_pi_multiple, sin_at_pi_multiple, tan_at_pi_multiple};

// ---------------------------------------------------------------------------
// Public evaluation functions
// ---------------------------------------------------------------------------

/// Evaluate `sin(arg)`.
///
/// # Examples
///
/// ```rust
/// use cas_trig::{sin_eval, PI};
/// use symbolic_ir::{apply, int, rat, sym, MUL};
///
/// // sin(0) = 0
/// assert_eq!(sin_eval(&int(0)), int(0));
///
/// // sin(π) = 0
/// assert_eq!(sin_eval(&sym(PI)), int(0));
///
/// // sin(π/2) = 1
/// let half_pi = apply(sym(MUL), vec![rat(1, 2), sym(PI)]);
/// assert_eq!(sin_eval(&half_pi), int(1));
/// ```
pub fn sin_eval(arg: &IRNode) -> IRNode {
    // Tier 1: exact special value.
    if let Some((n, d)) = extract_pi_multiple(arg) {
        if let Some(v) = sin_at_pi_multiple(n, d) {
            return v;
        }
    }
    // Tier 2: numeric.
    if let Some(v) = to_float(arg) {
        return sin_numeric(v);
    }
    // Tier 3: unevaluated.
    apply(sym(SIN), vec![arg.clone()])
}

/// Evaluate `cos(arg)`.
///
/// # Examples
///
/// ```rust
/// use cas_trig::{cos_eval, PI};
/// use symbolic_ir::{apply, int, rat, sym, MUL};
///
/// // cos(0) = 1
/// assert_eq!(cos_eval(&int(0)), int(1));
///
/// // cos(π) = -1
/// assert_eq!(cos_eval(&sym(PI)), int(-1));
///
/// // cos(π/3) = 1/2
/// let pi_3 = apply(sym(MUL), vec![rat(1, 3), sym(PI)]);
/// assert_eq!(cos_eval(&pi_3), rat(1, 2));
/// ```
pub fn cos_eval(arg: &IRNode) -> IRNode {
    if let Some((n, d)) = extract_pi_multiple(arg) {
        if let Some(v) = cos_at_pi_multiple(n, d) {
            return v;
        }
    }
    if let Some(v) = to_float(arg) {
        return cos_numeric(v);
    }
    apply(sym(COS), vec![arg.clone()])
}

/// Evaluate `tan(arg)`.
///
/// Returns unevaluated `Tan(arg)` at poles (π/2, 3π/2, …) and for
/// symbolic arguments.
///
/// # Examples
///
/// ```rust
/// use cas_trig::{tan_eval, PI};
/// use symbolic_ir::{apply, int, rat, sym, IRNode, MUL, TAN};
///
/// // tan(π/4) = 1
/// let pi_4 = apply(sym(MUL), vec![rat(1, 4), sym(PI)]);
/// assert_eq!(tan_eval(&pi_4), int(1));
///
/// // tan(π/2) = undefined → unevaluated
/// let pi_2 = apply(sym(MUL), vec![rat(1, 2), sym(PI)]);
/// assert!(matches!(tan_eval(&pi_2), IRNode::Apply(a) if a.head == sym(TAN)));
/// ```
pub fn tan_eval(arg: &IRNode) -> IRNode {
    if let Some((n, d)) = extract_pi_multiple(arg) {
        // tan_at_pi_multiple returns None for undefined angles (π/2, 3π/2)
        // and also None for unrecognised denominators.  Both cases fall
        // through to the unevaluated return below rather than producing ∞.
        if let Some(v) = tan_at_pi_multiple(n, d) {
            return v;
        }
        // If it's a recognised-but-undefined angle, return unevaluated.
        return apply(sym(TAN), vec![arg.clone()]);
    }
    if let Some(v) = to_float(arg) {
        let result = tan_numeric(v);
        // tan_numeric returns Float(f64::INFINITY) at poles — return unevaluated.
        if let IRNode::Float(f) = result {
            if f.is_infinite() {
                return apply(sym(TAN), vec![arg.clone()]);
            }
        }
        return result;
    }
    apply(sym(TAN), vec![arg.clone()])
}

/// Evaluate `atan(arg)`.
///
/// Returns a `Float` for numeric arguments, unevaluated `Atan(arg)` otherwise.
pub fn atan_eval(arg: &IRNode) -> IRNode {
    if let Some(v) = to_float(arg) {
        return atan_numeric(v);
    }
    apply(sym(ATAN), vec![arg.clone()])
}

/// Evaluate `asin(arg)`.
///
/// Returns `Float(asin(v))` for `v ∈ [−1, 1]`, unevaluated otherwise.
pub fn asin_eval(arg: &IRNode) -> IRNode {
    if let Some(v) = to_float(arg) {
        let result = asin_numeric(v);
        // asin_numeric returns NaN for |v| > 1 — return unevaluated.
        if let IRNode::Float(f) = result {
            if f.is_nan() {
                return apply(sym(ASIN), vec![arg.clone()]);
            }
        }
        return result;
    }
    apply(sym(ASIN), vec![arg.clone()])
}

/// Evaluate `acos(arg)`.
///
/// Returns `Float(acos(v))` for `v ∈ [−1, 1]`, unevaluated otherwise.
pub fn acos_eval(arg: &IRNode) -> IRNode {
    if let Some(v) = to_float(arg) {
        let result = acos_numeric(v);
        if let IRNode::Float(f) = result {
            if f.is_nan() {
                return apply(sym(ACOS), vec![arg.clone()]);
            }
        }
        return result;
    }
    apply(sym(ACOS), vec![arg.clone()])
}

/// Walk an expression tree and simplify every trig node.
///
/// Recursively simplifies sub-expressions bottom-up, then applies the
/// appropriate `*_eval` function when the head is `Sin`, `Cos`, `Tan`,
/// `Atan`, `Asin`, or `Acos`.
///
/// # Examples
///
/// ```rust
/// use cas_trig::{trig_simplify, PI};
/// use symbolic_ir::{apply, int, sym, ADD, COS, SIN};
///
/// // trig_simplify descends into Add(Sin(0), Cos(Pi))
/// // → Add(0, -1)  (both trig nodes evaluated)
/// let expr = apply(sym(ADD), vec![
///     apply(sym(SIN), vec![int(0)]),
///     apply(sym(COS), vec![sym(PI)]),
/// ]);
/// let result = trig_simplify(&expr);
/// if let symbolic_ir::IRNode::Apply(a) = &result {
///     assert_eq!(a.args[0], int(0));
///     assert_eq!(a.args[1], int(-1));
/// }
/// ```
pub fn trig_simplify(expr: &IRNode) -> IRNode {
    match expr {
        IRNode::Apply(a) => {
            // Bottom-up: simplify all arguments first.
            let simplified_args: Vec<IRNode> = a.args.iter().map(trig_simplify).collect();
            let head_name = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                other => {
                    // Non-symbol head: rebuild with simplified args.
                    return apply(other.clone(), simplified_args);
                }
            };
            match head_name {
                "Sin" if simplified_args.len() == 1 => sin_eval(&simplified_args[0]),
                "Cos" if simplified_args.len() == 1 => cos_eval(&simplified_args[0]),
                "Tan" if simplified_args.len() == 1 => tan_eval(&simplified_args[0]),
                "Atan" if simplified_args.len() == 1 => atan_eval(&simplified_args[0]),
                "Asin" if simplified_args.len() == 1 => asin_eval(&simplified_args[0]),
                "Acos" if simplified_args.len() == 1 => acos_eval(&simplified_args[0]),
                _ => apply(a.head.clone(), simplified_args),
            }
        }
        // Atoms are already fully simplified.
        _ => expr.clone(),
    }
}

// ---------------------------------------------------------------------------
// π-multiple recognition
// ---------------------------------------------------------------------------

/// Try to recognise `arg` as `(num/den) · π`.
///
/// Recognised patterns:
/// - `Integer(0)` → `(0, 1)` — zero equals `0 · π`
/// - `Symbol("Pi")` → `(1, 1)`
/// - `Neg(Symbol("Pi"))` → `(-1, 1)`
/// - `Mul(r, Pi)` or `Mul(Pi, r)` for rational `r` → `(r.num, r.den)`
/// - `Neg(Mul(r, Pi))` or `Neg(Mul(Pi, r))` → negated form
///
/// Returns `None` for all other shapes.
pub fn extract_pi_multiple(arg: &IRNode) -> Option<(i64, i64)> {
    match arg {
        // 0 = 0 * π
        IRNode::Integer(0) => Some((0, 1)),
        // Pi = 1 * π
        IRNode::Symbol(s) if s.as_str() == PI_SYMBOL => Some((1, 1)),
        IRNode::Apply(a) => {
            let head = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => return None,
            };
            match head {
                // Neg(x) → negate the fraction from x
                "Neg" if a.args.len() == 1 => {
                    let (n, d) = extract_pi_multiple(&a.args[0])?;
                    Some((-n, d))
                }
                // Mul(r, Pi) or Mul(Pi, r)
                "Mul" if a.args.len() == 2 => {
                    let (left, right) = (&a.args[0], &a.args[1]);
                    if is_pi(right) {
                        extract_rational(left)
                    } else if is_pi(left) {
                        extract_rational(right)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Return `true` iff `node` is `Symbol("Pi")`.
fn is_pi(node: &IRNode) -> bool {
    matches!(node, IRNode::Symbol(s) if s.as_str() == PI_SYMBOL)
}

/// Extract `(num, den)` from an `Integer` or `Rational` node.
fn extract_rational(n: &IRNode) -> Option<(i64, i64)> {
    match n {
        IRNode::Integer(v) => Some((*v, 1)),
        IRNode::Rational(num, den) => Some((*num, *den)),
        _ => None,
    }
}
