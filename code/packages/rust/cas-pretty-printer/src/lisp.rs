//! Always-prefix Lisp dialect — every node renders as `(Head args…)`.
//!
//! Useful for debugging the IR tree shape itself: no sugar, no infix
//! operators, no precedence — what you see is the raw tree.
//!
//! # Example
//!
//! ```text
//! Add(2, Mul(3, x))  →  (Add 2 (Mul 3 x))
//! ```
//!
//! # Two entry points
//!
//! - [`LispDialect`] — a [`Dialect`] implementation that disables all
//!   operator spellings so the walker falls through to function-call form
//!   everywhere.  It uses space-separated args inside `(…)`.
//!
//! - [`format_lisp`] — a standalone recursive function that bypasses the
//!   walker entirely.  It is the recommended entry point for Lisp output
//!   because it is guaranteed not to be affected by any
//!   [`register_head_formatter`](crate::register_head_formatter) calls and
//!   does not perform any sugar rewrites.

use symbolic_ir::{IRApply, IRNode};

use crate::dialect::{default_precedence, Dialect};

// ---------------------------------------------------------------------------
// LispDialect
// ---------------------------------------------------------------------------

/// Lisp/prefix dialect — no sugar, no infix operators.
///
/// Using `pretty(node, &LispDialect)` routes every `IRApply` through the
/// walker's function-call path, producing space-separated Lisp-style output.
/// For a fully isolated Lisp renderer that ignores registered head
/// formatters, prefer the [`format_lisp`] function instead.
pub struct LispDialect;

impl Dialect for LispDialect {
    fn name(&self) -> &str {
        "lisp"
    }

    fn format_integer(&self, value: i64) -> String {
        value.to_string()
    }

    fn format_rational(&self, numer: i64, denom: i64) -> String {
        format!("{}/{}", numer, denom)
    }

    fn format_float(&self, value: f64) -> String {
        format!("{:?}", value)
    }

    fn format_string(&self, value: &str) -> String {
        format!("\"{}\"", value)
    }

    fn format_symbol(&self, name: &str) -> String {
        name.to_string()
    }

    /// No binary operators — every head falls through to function-call form.
    fn binary_op(&self, _head_name: &str) -> Option<String> {
        None
    }

    /// No unary operators — every head falls through to function-call form.
    fn unary_op(&self, _head_name: &str) -> Option<String> {
        None
    }

    /// Function names are kept as-is (CamelCase head names).
    fn function_name(&self, head_name: &str) -> String {
        head_name.to_string()
    }

    /// Lists are rendered as `(List a b c)` — same prefix form as everything
    /// else in the Lisp dialect.
    fn list_brackets(&self) -> (&'static str, &'static str) {
        ("(", ")")
    }

    /// Function calls use Lisp-style space-separated args inside `(…)`.
    fn call_brackets(&self) -> (&'static str, &'static str) {
        ("(", ")")
    }

    fn precedence(&self, head_name: &str) -> u32 {
        default_precedence(head_name)
    }

    fn is_right_associative(&self, _head_name: &str) -> bool {
        false
    }

    /// No sugar — we want to see the raw IR tree.
    fn try_sugar(&self, _node: &IRApply) -> Option<IRNode> {
        None
    }
}

// ---------------------------------------------------------------------------
// Standalone format_lisp (bypasses walker entirely)
// ---------------------------------------------------------------------------

/// Format `node` as an always-prefix S-expression.
///
/// This function bypasses the walker entirely — no sugar, no registered
/// head formatters, no precedence.  Every `IRApply` becomes
/// `(Head arg1 arg2 …)`.
///
/// Prefer this over `pretty(node, &LispDialect)` when you want a
/// completely isolated debug representation.
///
/// # Example
///
/// ```rust
/// use cas_pretty_printer::format_lisp;
/// use symbolic_ir::{apply, int, sym, ADD, MUL};
///
/// let x = sym("x");
/// let expr = apply(sym(ADD), vec![
///     int(2),
///     apply(sym(MUL), vec![int(3), x]),
/// ]);
/// assert_eq!(format_lisp(&expr), "(Add 2 (Mul 3 x))");
/// ```
pub fn format_lisp(node: &IRNode) -> String {
    match node {
        // Leaf nodes — rendered the same way in every dialect.
        IRNode::Integer(v) => v.to_string(),
        IRNode::Rational(n, d) => format!("{}/{}", n, d),
        IRNode::Float(v) => format!("{:?}", v),
        IRNode::Str(s) => format!("\"{}\"", s),
        IRNode::Symbol(name) => name.clone(),

        // Compound: `(Head arg1 arg2 …)` or just `(Head)` for 0-arg nodes.
        IRNode::Apply(a) => {
            let head_text = format_lisp(&a.head);
            if a.args.is_empty() {
                format!("({})", head_text)
            } else {
                let args_text = a
                    .args
                    .iter()
                    .map(format_lisp)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({} {})", head_text, args_text)
            }
        }
    }
}
