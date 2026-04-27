//! Power-reduction identities for sin² and cos².
//!
//! The half-angle identities rewrite squared trig functions into expressions
//! involving `cos(2x)`, removing the power:
//!
//! ```text
//! sin²(x) = (1 − cos(2x)) / 2  =  Mul(1/2, Sub(1, Cos(Mul(2, x))))
//! cos²(x) = (1 + cos(2x)) / 2  =  Mul(1/2, Add(1, Cos(Mul(2, x))))
//! ```
//!
//! ## Derivation
//!
//! Both identities follow from the double-angle formula:
//!
//! ```text
//! cos(2x) = 1 − 2·sin²(x)   ⟹   sin²(x) = (1 − cos(2x)) / 2
//! cos(2x) = 2·cos²(x) − 1   ⟹   cos²(x) = (1 + cos(2x)) / 2
//! ```
//!
//! ## Scope
//!
//! `power_reduce` applies only to `Pow(Sin(x), 2)` and `Pow(Cos(x), 2)`.
//! Higher powers (e.g. `sin⁴(x)`) can be reduced by applying `power_reduce`
//! multiple times, but the caller is responsible for iterating.  (Phase 5b
//! of the integration roadmap handles the general `sinⁿ` case via the IBP
//! reduction formula.)
//!
//! The function traverses the full expression tree recursively so that nested
//! trig powers inside a larger expression are also reduced.

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, COS, MUL, SUB};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Walk `expr` and reduce `sin²(x)` and `cos²(x)` to half-angle forms.
///
/// Applied to `Pow(Sin(…), 2)` → `Mul(1/2, Sub(1, Cos(Mul(2, …))))`,
/// and to `Pow(Cos(…), 2)` → `Mul(1/2, Add(1, Cos(Mul(2, …))))`.
///
/// All other expressions are recursed into unchanged.
///
/// # Examples
///
/// ```rust
/// use cas_trig::power_reduce;
/// use symbolic_ir::{apply, int, rat, sym, ADD, COS, MUL, POW, SIN, SUB};
///
/// // sin²(x) → (1 − cos(2x)) / 2
/// let sin_sq = apply(sym(POW), vec![
///     apply(sym(SIN), vec![sym("x")]),
///     int(2),
/// ]);
/// let reduced = power_reduce(&sin_sq);
/// let expected = apply(sym(MUL), vec![
///     rat(1, 2),
///     apply(sym(SUB), vec![int(1), apply(sym(COS), vec![
///         apply(sym(MUL), vec![int(2), sym("x")])])])
/// ]);
/// assert_eq!(reduced, expected);
/// ```
pub fn power_reduce(expr: &IRNode) -> IRNode {
    match expr {
        IRNode::Apply(a) => {
            let head = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => {
                    let reduced: Vec<_> = a.args.iter().map(power_reduce).collect();
                    return apply(a.head.clone(), reduced);
                }
            };

            // Pow(…, 2): check whether the base is Sin or Cos
            if head == "Pow" && a.args.len() == 2 {
                if a.args[1] == IRNode::Integer(2) {
                    if let Some(inner) = extract_sin_arg(&a.args[0]) {
                        return sin_squared(inner);
                    }
                    if let Some(inner) = extract_cos_arg(&a.args[0]) {
                        return cos_squared(inner);
                    }
                }
            }

            // Otherwise: recurse into args
            let reduced: Vec<_> = a.args.iter().map(power_reduce).collect();
            apply(a.head.clone(), reduced)
        }
        _ => expr.clone(),
    }
}

// ---------------------------------------------------------------------------
// Reduction builders
// ---------------------------------------------------------------------------

/// `sin²(inner)` → `Mul(1/2, Sub(1, Cos(Mul(2, inner))))`
fn sin_squared(inner: &IRNode) -> IRNode {
    // First reduce any trig powers inside `inner` as well.
    let inner_r = power_reduce(inner);
    let double_arg = apply(sym(MUL), vec![int(2), inner_r]);
    let cos_2x = apply(sym(COS), vec![double_arg]);
    let numer = apply(sym(SUB), vec![int(1), cos_2x]);
    apply(sym(MUL), vec![rat(1, 2), numer])
}

/// `cos²(inner)` → `Mul(1/2, Add(1, Cos(Mul(2, inner))))`
fn cos_squared(inner: &IRNode) -> IRNode {
    let inner_r = power_reduce(inner);
    let double_arg = apply(sym(MUL), vec![int(2), inner_r]);
    let cos_2x = apply(sym(COS), vec![double_arg]);
    let numer = apply(sym(ADD), vec![int(1), cos_2x]);
    apply(sym(MUL), vec![rat(1, 2), numer])
}

// ---------------------------------------------------------------------------
// Pattern helpers
// ---------------------------------------------------------------------------

/// If `node` is `Sin(inner)`, return `Some(inner)`.
fn extract_sin_arg(node: &IRNode) -> Option<&IRNode> {
    if let IRNode::Apply(a) = node {
        if matches!(&a.head, IRNode::Symbol(s) if s.as_str() == "Sin") && a.args.len() == 1 {
            return Some(&a.args[0]);
        }
    }
    None
}

/// If `node` is `Cos(inner)`, return `Some(inner)`.
fn extract_cos_arg(node: &IRNode) -> Option<&IRNode> {
    if let IRNode::Apply(a) = node {
        if matches!(&a.head, IRNode::Symbol(s) if s.as_str() == "Cos") && a.args.len() == 1 {
            return Some(&a.args[0]);
        }
    }
    None
}
