//! Mathematica dialect.
//!
//! Surface syntax differences from MACSYMA:
//!
//! - Function calls use square brackets: `Sin[x]`, `D[f, x]`.
//! - Lists use curly braces: `{1, 2, 3}`.
//! - Function names keep the IR's CamelCase exactly: `Sin`, `Cos`, `Limit`.
//! - Equality is `==` (not `=`).
//! - Not-equal is `!=` (not `#`).
//! - Logical `&&` / `||` / `!` rather than `and` / `or` / `not `.
//!
//! Arithmetic surface sugar (Neg, Sub, Div) is shared with MACSYMA via
//! [`macsyma_sugar`].

use symbolic_ir::{IRApply, IRNode};

use crate::dialect::{
    default_binary_op, default_precedence, default_unary_op, Dialect,
};
use crate::macsyma::macsyma_sugar;

// ---------------------------------------------------------------------------
// MathematicaDialect
// ---------------------------------------------------------------------------

/// Mathematica dialect.
///
/// Use `&MathematicaDialect` when passing to [`pretty`](crate::pretty).
pub struct MathematicaDialect;

impl Dialect for MathematicaDialect {
    fn name(&self) -> &str {
        "mathematica"
    }

    // ---- numeric formatting ------------------------------------------------
    // Identical to MACSYMA — Mathematica uses the same decimal notation.

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

    // ---- operators ---------------------------------------------------------
    // Override the handful of spellings that differ from MACSYMA defaults;
    // delegate everything else to the shared table.

    fn binary_op(&self, head_name: &str) -> Option<String> {
        match head_name {
            // Mathematica uses `==` for equality and `!=` for not-equal.
            "Equal" => Some(" == ".to_string()),
            "NotEqual" => Some(" != ".to_string()),
            // Mathematica uses `&&` / `||` for logic.
            "And" => Some(" && ".to_string()),
            "Or" => Some(" || ".to_string()),
            other => default_binary_op(other),
        }
    }

    fn unary_op(&self, head_name: &str) -> Option<String> {
        match head_name {
            // Mathematica uses `!` for logical Not.
            "Not" => Some("!".to_string()),
            other => default_unary_op(other),
        }
    }

    /// In Mathematica, function names keep the IR's CamelCase exactly:
    /// `Sin`, `Cos`, `D`, `Limit`, …  No lowercase aliasing.
    fn function_name(&self, head_name: &str) -> String {
        head_name.to_string()
    }

    // ---- containers --------------------------------------------------------

    /// Lists use `{…}` in Mathematica: `{1, 2, 3}`.
    fn list_brackets(&self) -> (&'static str, &'static str) {
        ("{", "}")
    }

    /// Function calls use `[…]` in Mathematica: `Sin[x]`.
    fn call_brackets(&self) -> (&'static str, &'static str) {
        ("[", "]")
    }

    // ---- precedence --------------------------------------------------------

    fn precedence(&self, head_name: &str) -> u32 {
        default_precedence(head_name)
    }

    fn is_right_associative(&self, head_name: &str) -> bool {
        head_name == "Pow"
    }

    // ---- sugar -------------------------------------------------------------
    // Re-use MACSYMA sugar for arithmetic (Neg, Sub, Div from Inv).

    fn try_sugar(&self, node: &IRApply) -> Option<IRNode> {
        macsyma_sugar(node)
    }
}
