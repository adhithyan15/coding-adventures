//! Direct-substitution limit.
//!
//! `limit_direct(expr, var, point)` computes `lim_{var → point} expr` by
//! substituting `point` for every occurrence of `var` in `expr` using
//! [`cas_substitution::subst`].
//!
//! ## No simplification
//!
//! The substituted result is returned as-is (un-simplified).  For
//! `lim_{x→5} x + 0`, the result is `Add(5, 0)` not `5`.  The caller
//! should pass the result through `cas_simplify::simplify` when needed.
//!
//! ## Indeterminate form detection
//!
//! The only automatically detected indeterminate form is a literal
//! `Div(Integer(0), Integer(0))`.  When this arises after substitution, the
//! function returns the unevaluated wrapper `Limit(expr, var, point)` so the
//! caller's VM knows that L'Hôpital or another strategy is needed.
//!
//! Detecting `Add(x, Inv(0))` or `∞/∞` forms requires simplification, which
//! this layer deliberately avoids.

use symbolic_ir::{apply, sym, IRNode, DIV};

use cas_substitution::subst;

use crate::LIMIT;

/// Compute `lim_{var → point} expr` by direct substitution.
///
/// Returns the substituted expression (un-simplified).  If the substituted
/// result is the literal `Div(0, 0)`, returns `Limit(expr, var, point)`
/// instead.
///
/// ```rust
/// use cas_limit_series::limit_direct;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// let x = sym("x");
///
/// // lim_{x→3} 2*x  →  Mul(2, 3)
/// let expr = apply(sym(MUL), vec![int(2), x.clone()]);
/// assert_eq!(
///     limit_direct(expr, &x, int(3)),
///     apply(sym(MUL), vec![int(2), int(3)])
/// );
///
/// // lim_{x→0} (x + 0)  →  Add(0, 0)  (un-simplified)
/// let x2 = sym("x");
/// let expr2 = apply(sym(ADD), vec![x2.clone(), symbolic_ir::int(0)]);
/// let out = limit_direct(expr2, &x2, symbolic_ir::int(0));
/// assert_eq!(out, apply(sym(ADD), vec![symbolic_ir::int(0), symbolic_ir::int(0)]));
/// ```
pub fn limit_direct(expr: IRNode, var: &IRNode, point: IRNode) -> IRNode {
    let out = subst(point.clone(), var, expr.clone());
    if looks_indeterminate(&out) {
        // Build Limit(expr, var, point) as an unevaluated Apply node.
        apply(sym(LIMIT), vec![expr, var.clone(), point])
    } else {
        out
    }
}

/// Conservatively detect a `Div(Integer(0), Integer(0))` literal.
///
/// Only catches the simplest case: both numerator and denominator are already
/// the integer literal 0.  Mixed forms (`Div(Sub(x,x), 0)`, etc.) require
/// simplification and are left to the caller.
fn looks_indeterminate(node: &IRNode) -> bool {
    if let IRNode::Apply(a) = node {
        if a.head == sym(DIV) && a.args.len() == 2 {
            if let (IRNode::Integer(0), IRNode::Integer(0)) = (&a.args[0], &a.args[1]) {
                return true;
            }
        }
    }
    false
}
