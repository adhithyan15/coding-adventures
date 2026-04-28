//! MACSYMA / Maxima dialect.
//!
//! Surface syntax conventions:
//!
//! - Function calls use parentheses: `sin(x)`, `f(x, y)`.
//! - Lists use square brackets: `[1, 2, 3]`.
//! - Power is `^` (MACSYMA also accepts `**` on input; output uses `^`).
//! - Equality is `=`; not-equal is `#`.
//! - Function names are lowercase: `sin`, `cos`, `log`, `exp`, `diff`,
//!   `integrate`.
//!
//! # Surface sugar
//!
//! The MACSYMA dialect applies three rewrite rules before the walker
//! dispatches:
//!
//! | Input IR                       | Rewrites to  | Displays as |
//! |-------------------------------|--------------|-------------|
//! | `Mul(-1, x)`                  | `Neg(x)`     | `-x`        |
//! | `Mul(-1, x, y, …)`            | `Neg(Mul(x, y, …))` | `-x*y*…` |
//! | `Add(a, Neg(b))` _(2-arg)_   | `Sub(a, b)`  | `a - b`     |
//! | `Mul(a, Inv(b))` _(2-arg)_   | `Div(a, b)`  | `a/b`       |
//!
//! The walker applies sugar recursively, so `Mul(-1, Inv(y))` → `Neg(Inv(y))`
//! and then formats as `-Inv(y)` (Inv has no infix form by default).

use symbolic_ir::{apply, sym, IRApply, IRNode, DIV, INV, MUL, NEG, SUB};

use crate::dialect::{
    default_binary_op, default_function_name, default_precedence, default_unary_op, Dialect,
};

// ---------------------------------------------------------------------------
// MacsymaDialect
// ---------------------------------------------------------------------------

/// MACSYMA/Maxima dialect.
///
/// Use `&MacsymaDialect` when passing to [`pretty`](crate::pretty).
pub struct MacsymaDialect;

impl Dialect for MacsymaDialect {
    fn name(&self) -> &str {
        "macsyma"
    }

    // ---- numeric formatting ------------------------------------------------

    fn format_integer(&self, value: i64) -> String {
        value.to_string()
    }

    fn format_rational(&self, numer: i64, denom: i64) -> String {
        format!("{}/{}", numer, denom)
    }

    /// Uses Rust's `{:?}` which produces round-trip–safe output (e.g.
    /// `3.14` stays `3.14`, not `3.1400000000000001`).
    fn format_float(&self, value: f64) -> String {
        format!("{:?}", value)
    }

    fn format_string(&self, value: &str) -> String {
        format!("\"{}\"", value)
    }

    fn format_symbol(&self, name: &str) -> String {
        name.to_string()
    }

    // ---- operators ---------------------------------------------------------

    fn binary_op(&self, head_name: &str) -> Option<String> {
        default_binary_op(head_name)
    }

    fn unary_op(&self, head_name: &str) -> Option<String> {
        default_unary_op(head_name)
    }

    fn function_name(&self, head_name: &str) -> String {
        default_function_name(head_name)
    }

    // ---- containers --------------------------------------------------------

    fn list_brackets(&self) -> (&'static str, &'static str) {
        ("[", "]")
    }

    fn call_brackets(&self) -> (&'static str, &'static str) {
        ("(", ")")
    }

    // ---- precedence --------------------------------------------------------

    fn precedence(&self, head_name: &str) -> u32 {
        default_precedence(head_name)
    }

    fn is_right_associative(&self, head_name: &str) -> bool {
        // Only `^` is right-associative: `a^b^c` = `a^(b^c)`.
        head_name == "Pow"
    }

    // ---- sugar -------------------------------------------------------------

    fn try_sugar(&self, node: &IRApply) -> Option<IRNode> {
        macsyma_sugar(node)
    }
}

// ---------------------------------------------------------------------------
// Sugar logic (also used by MathematicaDialect and MapleDialect)
// ---------------------------------------------------------------------------

/// Apply MACSYMA surface-sugar rules to `node`.
///
/// Called by both [`MacsymaDialect`] and the other dialects that share the
/// same arithmetic surface syntax.
pub(crate) fn macsyma_sugar(node: &IRApply) -> Option<IRNode> {
    let head_name = match &node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return None,
    };

    // Rule 1: Mul(-1, x) → Neg(x)
    //         Mul(-1, x, y, …) → Neg(Mul(x, y, …))
    //
    // This turns `Mul(-1, x)` into `-x` via the Neg unary op.
    if head_name == MUL && node.args.len() >= 2 {
        if let IRNode::Integer(-1) = &node.args[0] {
            let rest = node.args[1..].to_vec();
            let inner = if rest.len() == 1 {
                rest.into_iter().next().unwrap()
            } else {
                apply(sym(MUL), rest)
            };
            return Some(apply(sym(NEG), vec![inner]));
        }
    }

    // Rule 2: Add(a, Neg(b)) → Sub(a, b)  [2-arg case only]
    //
    // Only the trailing-negated-argument case is sugar'd; multi-negative
    // Add expressions are left to the generic Add infix rule.
    if head_name == "Add" && node.args.len() == 2 {
        let (a, b) = (&node.args[0], &node.args[1]);
        if let IRNode::Apply(b_apply) = b {
            if let IRNode::Symbol(b_head) = &b_apply.head {
                if b_head.as_str() == NEG && b_apply.args.len() == 1 {
                    return Some(apply(
                        sym(SUB),
                        vec![a.clone(), b_apply.args[0].clone()],
                    ));
                }
            }
        }
    }

    // Rule 3: Mul(a, Inv(b)) → Div(a, b)  [2-arg case only]
    //
    // Same caution as Rule 2: only the simple 2-arg multiplication by an
    // inverse is sugar'd.
    if head_name == MUL && node.args.len() == 2 {
        let (a, b) = (&node.args[0], &node.args[1]);
        if let IRNode::Apply(b_apply) = b {
            if let IRNode::Symbol(b_head) = &b_apply.head {
                if b_head.as_str() == INV && b_apply.args.len() == 1 {
                    return Some(apply(
                        sym(DIV),
                        vec![a.clone(), b_apply.args[0].clone()],
                    ));
                }
            }
        }
    }

    None
}
