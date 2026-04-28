//! Angle-addition formula expansion for trig expressions.
//!
//! This module expands `sin(a ± b)` and `cos(a ± b)` using the standard
//! angle-addition identities:
//!
//! ```text
//! sin(a + b) = sin(a)·cos(b) + cos(a)·sin(b)
//! cos(a + b) = cos(a)·cos(b) − sin(a)·sin(b)
//! sin(a − b) = sin(a)·cos(b) − cos(a)·sin(b)
//! cos(a − b) = cos(a)·cos(b) + sin(a)·sin(b)
//! sin(−a)    = −sin(a)
//! cos(−a)    = cos(a)
//! ```
//!
//! Double-angle special cases are also handled:
//!
//! ```text
//! sin(2·a) = 2·sin(a)·cos(a)
//! cos(2·a) = cos²(a) − sin²(a)
//! ```
//!
//! ## Opt-in expansion
//!
//! Expansion is **not** applied automatically by `trig_simplify` — it must
//! be requested explicitly via [`expand_trig`].  Automatic expansion would
//! turn simple expressions like `sin(x + π)` into the longer (though
//! equivalent) form `sin(x)·cos(π) + cos(x)·sin(π)`.
//!
//! ## Recursive application
//!
//! Expansion is applied top-down: outer trig functions are expanded first,
//! and the resulting sub-expressions are then expanded recursively.

use symbolic_ir::{apply, int, sym, IRNode, ADD, COS, MUL, NEG, SIN, SUB};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Walk `expr` and expand `sin(a ± b)` and `cos(a ± b)` via angle-addition.
///
/// The expansion is applied to every `Sin`/`Cos` node whose argument is a
/// `Sub`, `Add`, or `Neg` expression.  Double-angle forms (`sin(2·a)`,
/// `cos(2·a)`) are also expanded.
///
/// Non-trig sub-expressions are traversed recursively so that inner trig
/// nodes are expanded even inside `Add`, `Mul`, etc.
///
/// # Examples
///
/// ```rust
/// use cas_trig::expand_trig;
/// use symbolic_ir::{apply, sym, ADD, COS, MUL, SIN, SUB};
///
/// // sin(x + y) → sin(x)·cos(y) + cos(x)·sin(y)
/// let expr = apply(sym(SIN), vec![
///     apply(sym(ADD), vec![sym("x"), sym("y")])
/// ]);
/// let expanded = expand_trig(&expr);
/// let expected = apply(sym(ADD), vec![
///     apply(sym(MUL), vec![
///         apply(sym(SIN), vec![sym("x")]),
///         apply(sym(COS), vec![sym("y")]),
///     ]),
///     apply(sym(MUL), vec![
///         apply(sym(COS), vec![sym("x")]),
///         apply(sym(SIN), vec![sym("y")]),
///     ]),
/// ]);
/// assert_eq!(expanded, expected);
/// ```
pub fn expand_trig(expr: &IRNode) -> IRNode {
    match expr {
        IRNode::Apply(a) => {
            let head = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => {
                    let args: Vec<_> = a.args.iter().map(expand_trig).collect();
                    return apply(a.head.clone(), args);
                }
            };
            match head {
                "Sin" if a.args.len() == 1 => expand_sin(&a.args[0]),
                "Cos" if a.args.len() == 1 => expand_cos(&a.args[0]),
                _ => {
                    let args: Vec<_> = a.args.iter().map(expand_trig).collect();
                    apply(a.head.clone(), args)
                }
            }
        }
        _ => expr.clone(),
    }
}

// ---------------------------------------------------------------------------
// Internal expansion functions
// ---------------------------------------------------------------------------

/// Expand `sin(arg)` using angle-addition rules.
fn expand_sin(arg: &IRNode) -> IRNode {
    match arg {
        // sin(a + b) = sin(a)·cos(b) + cos(a)·sin(b)
        IRNode::Apply(a) if is_head(&a.head, "Add") && a.args.len() == 2 => {
            let (a_node, b_node) = (&a.args[0], &a.args[1]);
            let sin_a = expand_sin(a_node);
            let cos_b = expand_cos(b_node);
            let cos_a = expand_cos(a_node);
            let sin_b = expand_sin(b_node);
            add(mul(sin_a, cos_b), mul(cos_a, sin_b))
        }
        // sin(a − b) = sin(a)·cos(b) − cos(a)·sin(b)
        IRNode::Apply(a) if is_head(&a.head, "Sub") && a.args.len() == 2 => {
            let (a_node, b_node) = (&a.args[0], &a.args[1]);
            let sin_a = expand_sin(a_node);
            let cos_b = expand_cos(b_node);
            let cos_a = expand_cos(a_node);
            let sin_b = expand_sin(b_node);
            sub(mul(sin_a, cos_b), mul(cos_a, sin_b))
        }
        // sin(−a) = −sin(a)
        IRNode::Apply(a) if is_head(&a.head, "Neg") && a.args.len() == 1 => {
            neg(expand_sin(&a.args[0]))
        }
        // sin(2·a) = 2·sin(a)·cos(a)  — check Mul(2, a) or Mul(a, 2)
        IRNode::Apply(a) if is_head(&a.head, "Mul") && a.args.len() == 2 => {
            if let Some(inner) = extract_double_angle(&a.args) {
                mul(int(2), mul(expand_sin(inner), expand_cos(inner)))
            } else {
                // Not a double-angle: recurse into arg but keep Sin head
                apply(sym(SIN), vec![expand_trig(arg)])
            }
        }
        // General: recurse into the argument (handles nested trig)
        _ => apply(sym(SIN), vec![expand_trig(arg)]),
    }
}

/// Expand `cos(arg)` using angle-addition rules.
fn expand_cos(arg: &IRNode) -> IRNode {
    match arg {
        // cos(a + b) = cos(a)·cos(b) − sin(a)·sin(b)
        IRNode::Apply(a) if is_head(&a.head, "Add") && a.args.len() == 2 => {
            let (a_node, b_node) = (&a.args[0], &a.args[1]);
            let cos_a = expand_cos(a_node);
            let cos_b = expand_cos(b_node);
            let sin_a = expand_sin(a_node);
            let sin_b = expand_sin(b_node);
            sub(mul(cos_a, cos_b), mul(sin_a, sin_b))
        }
        // cos(a − b) = cos(a)·cos(b) + sin(a)·sin(b)
        IRNode::Apply(a) if is_head(&a.head, "Sub") && a.args.len() == 2 => {
            let (a_node, b_node) = (&a.args[0], &a.args[1]);
            let cos_a = expand_cos(a_node);
            let cos_b = expand_cos(b_node);
            let sin_a = expand_sin(a_node);
            let sin_b = expand_sin(b_node);
            add(mul(cos_a, cos_b), mul(sin_a, sin_b))
        }
        // cos(−a) = cos(a)
        IRNode::Apply(a) if is_head(&a.head, "Neg") && a.args.len() == 1 => {
            expand_cos(&a.args[0])
        }
        // cos(2·a) = cos²(a) − sin²(a)
        IRNode::Apply(a) if is_head(&a.head, "Mul") && a.args.len() == 2 => {
            if let Some(inner) = extract_double_angle(&a.args) {
                let c = expand_cos(inner);
                let s = expand_sin(inner);
                // cos²(a) − sin²(a)
                sub(mul(c.clone(), c), mul(s.clone(), s))
            } else {
                apply(sym(COS), vec![expand_trig(arg)])
            }
        }
        _ => apply(sym(COS), vec![expand_trig(arg)]),
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn is_head(head: &IRNode, name: &str) -> bool {
    matches!(head, IRNode::Symbol(s) if s.as_str() == name)
}

/// If `args` is `[Integer(2), x]` or `[x, Integer(2)]`, return `Some(x)`.
fn extract_double_angle(args: &[IRNode]) -> Option<&IRNode> {
    match args {
        [IRNode::Integer(2), x] | [x, IRNode::Integer(2)] => Some(x),
        _ => None,
    }
}

fn add(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(ADD), vec![a, b])
}

fn sub(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(SUB), vec![a, b])
}

fn mul(a: IRNode, b: IRNode) -> IRNode {
    apply(sym(MUL), vec![a, b])
}

fn neg(a: IRNode) -> IRNode {
    apply(sym(NEG), vec![a])
}
