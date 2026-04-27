//! Maple dialect.
//!
//! Maple's surface syntax is very close to MACSYMA.  The one spelling
//! that differs in the set of operators implemented here is not-equal:
//! Maple uses `<>` where MACSYMA uses `#`.
//!
//! Everything else — function call syntax, list brackets, arithmetic
//! operators, and surface sugar — is shared with MACSYMA.

use symbolic_ir::{IRApply, IRNode};

use crate::dialect::{default_binary_op, default_function_name, default_precedence,
    default_unary_op, Dialect};
use crate::macsyma::macsyma_sugar;

// ---------------------------------------------------------------------------
// MapleDialect
// ---------------------------------------------------------------------------

/// Maple dialect.
///
/// Use `&MapleDialect` when passing to [`pretty`](crate::pretty).
pub struct MapleDialect;

impl Dialect for MapleDialect {
    fn name(&self) -> &str {
        "maple"
    }

    // ---- numeric formatting ------------------------------------------------
    // Identical to MACSYMA.

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

    fn binary_op(&self, head_name: &str) -> Option<String> {
        match head_name {
            // Maple uses `<>` for not-equal rather than MACSYMA's `#`.
            "NotEqual" => Some(" <> ".to_string()),
            other => default_binary_op(other),
        }
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
        head_name == "Pow"
    }

    // ---- sugar -------------------------------------------------------------

    fn try_sugar(&self, node: &IRApply) -> Option<IRNode> {
        macsyma_sugar(node)
    }
}
