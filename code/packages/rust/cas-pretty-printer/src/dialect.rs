//! The [`Dialect`] trait and default precedence / operator tables.
//!
//! A dialect tells the walker how to spell things in a particular CAS
//! language.  It provides:
//!
//! - numeric leaf formats (integers, rationals, floats, strings),
//! - operator spellings for binary and unary heads,
//! - function-call naming (e.g. `"D"` → `"diff"` in MACSYMA),
//! - container delimiters for lists and function calls,
//! - a precedence table — used by the walker to decide where parens go.
//!
//! Most dialects share the same default tables.  The free functions
//! [`default_binary_op`], [`default_unary_op`], [`default_function_name`],
//! and [`default_precedence`] implement those shared defaults; each concrete
//! dialect struct delegates to them and overrides only what differs.
//!
//! # Precedence levels (higher = tighter binding)
//!
//! ```text
//! PREC_OR   = 10    — logical Or
//! PREC_AND  = 20    — logical And
//! PREC_NOT  = 25    — logical Not
//! PREC_CMP  = 30    — =, ≠, <, >, ≤, ≥
//! PREC_ADD  = 40    — + and −
//! PREC_MUL  = 50    — × and ÷
//! PREC_NEG  = 55    — unary −
//! PREC_POW  = 60    — ^ / **
//! PREC_CALL = 70    — function call (default for unknown heads)
//! PREC_ATOM = 100   — leaf nodes
//! ```

use symbolic_ir::{IRApply, IRNode};

// ---------------------------------------------------------------------------
// Precedence level constants
// ---------------------------------------------------------------------------

/// Precedence of logical Or.
pub const PREC_OR: u32 = 10;
/// Precedence of logical And.
pub const PREC_AND: u32 = 20;
/// Precedence of logical Not.
pub const PREC_NOT: u32 = 25;
/// Precedence of comparison operators (=, ≠, <, >, ≤, ≥).
pub const PREC_CMP: u32 = 30;
/// Precedence of additive operators (+ and −).
pub const PREC_ADD: u32 = 40;
/// Precedence of multiplicative operators (× and ÷).
pub const PREC_MUL: u32 = 50;
/// Precedence of unary negation.
pub const PREC_NEG: u32 = 55;
/// Precedence of exponentiation.
pub const PREC_POW: u32 = 60;
/// Default precedence for unknown heads (function-call level).
pub const PREC_CALL: u32 = 70;
/// Precedence of leaf atoms.
pub const PREC_ATOM: u32 = 100;

// ---------------------------------------------------------------------------
// Default tables (shared across most dialects)
// ---------------------------------------------------------------------------

/// Return the infix operator string for `head_name`, or `None` to fall
/// through to function-call form.
///
/// This is the canonical table shared by MACSYMA, Maple, and (mostly)
/// Mathematica.  Individual dialects override the entries they need.
pub fn default_binary_op(head_name: &str) -> Option<String> {
    match head_name {
        "Add" => Some(" + ".to_string()),
        "Sub" => Some(" - ".to_string()),
        "Mul" => Some("*".to_string()),
        "Div" => Some("/".to_string()),
        "Pow" => Some("^".to_string()),
        "Or" => Some(" or ".to_string()),
        "And" => Some(" and ".to_string()),
        "Equal" => Some(" = ".to_string()),
        "NotEqual" => Some(" # ".to_string()),
        "Less" => Some(" < ".to_string()),
        "Greater" => Some(" > ".to_string()),
        "LessEqual" => Some(" <= ".to_string()),
        "GreaterEqual" => Some(" >= ".to_string()),
        _ => None,
    }
}

/// Return the prefix operator string for `head_name`, or `None`.
///
/// Default table: `"Neg"` → `"-"`, `"Not"` → `"not "`.
pub fn default_unary_op(head_name: &str) -> Option<String> {
    match head_name {
        "Neg" => Some("-".to_string()),
        "Not" => Some("not ".to_string()),
        _ => None,
    }
}

/// Translate an IR head name to its surface function name.
///
/// Returns the canonical lowercase MACSYMA / Maple name for built-in
/// functions.  Unknown heads are returned as-is.
pub fn default_function_name(head_name: &str) -> String {
    match head_name {
        "Sin" => "sin",
        "Cos" => "cos",
        "Tan" => "tan",
        "Exp" => "exp",
        "Log" => "log",
        "Sqrt" => "sqrt",
        "Abs" => "abs",
        "Asin" => "asin",
        "Acos" => "acos",
        "Atan" => "atan",
        "Sinh" => "sinh",
        "Cosh" => "cosh",
        "Tanh" => "tanh",
        "Asinh" => "asinh",
        "Acosh" => "acosh",
        "Atanh" => "atanh",
        "D" => "diff",
        "Integrate" => "integrate",
        "Simplify" => "simplify",
        "Expand" => "expand",
        "Factor" => "factor",
        "Subst" => "subst",
        "Solve" => "solve",
        "Limit" => "limit",
        "Taylor" => "taylor",
        "Length" => "length",
        "First" => "first",
        "Rest" => "rest",
        "Map" => "map",
        "Apply" => "apply",
        other => return other.to_string(),
    }
    .to_string()
}

/// Return the precedence level for `head_name`.
///
/// Unknown heads are treated as function calls (tightest binding).
pub fn default_precedence(head_name: &str) -> u32 {
    match head_name {
        "Or" => PREC_OR,
        "And" => PREC_AND,
        "Not" => PREC_NOT,
        "Equal" | "NotEqual" | "Less" | "Greater" | "LessEqual" | "GreaterEqual" => PREC_CMP,
        "Add" | "Sub" => PREC_ADD,
        "Mul" | "Div" => PREC_MUL,
        "Neg" => PREC_NEG,
        "Pow" => PREC_POW,
        _ => PREC_CALL,
    }
}

// ---------------------------------------------------------------------------
// Dialect trait
// ---------------------------------------------------------------------------

/// The minimal contract every dialect must satisfy.
///
/// Implementers usually define a unit struct (e.g. `pub struct MacsymaDialect`)
/// and delegate the common methods to the free functions above.
///
/// The walker is fully dialect-agnostic: it calls only these methods and
/// never inspects the concrete type behind the trait object.
pub trait Dialect: Send + Sync {
    /// Short name for debugging (e.g. `"macsyma"`, `"mathematica"`).
    fn name(&self) -> &str;

    // ------------------------------------------------------------------
    // Numeric leaf formatting
    // ------------------------------------------------------------------

    /// Format an `Integer` node value as a string.
    fn format_integer(&self, value: i64) -> String;

    /// Format a `Rational` node as a string.
    ///
    /// The numerator and denominator are already in lowest terms with
    /// `denom > 0`.
    fn format_rational(&self, numer: i64, denom: i64) -> String;

    /// Format a `Float` node value as a string.
    ///
    /// Should produce a round-trip–safe representation (Rust's `{:?}` for
    /// `f64` satisfies this).
    fn format_float(&self, value: f64) -> String;

    /// Format a `Str` node value, including any surrounding quotes.
    fn format_string(&self, value: &str) -> String;

    /// Format a `Symbol` node name.
    fn format_symbol(&self, name: &str) -> String;

    // ------------------------------------------------------------------
    // Operator spellings
    // ------------------------------------------------------------------

    /// Return the infix spelling for `head_name`, e.g. `" + "` for `"Add"`.
    ///
    /// Return `None` to fall through to function-call form.
    fn binary_op(&self, head_name: &str) -> Option<String>;

    /// Return the prefix spelling for `head_name`, e.g. `"-"` for `"Neg"`.
    ///
    /// Return `None` to fall through to function-call form.
    fn unary_op(&self, head_name: &str) -> Option<String>;

    // ------------------------------------------------------------------
    // Function-call spelling
    // ------------------------------------------------------------------

    /// Translate an IR head name to the surface-level function name.
    ///
    /// E.g. `"D"` → `"diff"` in MACSYMA; `"Sin"` → `"Sin"` in Mathematica.
    fn function_name(&self, head_name: &str) -> String;

    // ------------------------------------------------------------------
    // Container delimiters — return (open, close) pairs
    // ------------------------------------------------------------------

    /// Brackets used to delimit list literals, e.g. `("[", "]")`.
    fn list_brackets(&self) -> (&'static str, &'static str);

    /// Brackets used for function-call argument lists, e.g. `("(", ")")`.
    fn call_brackets(&self) -> (&'static str, &'static str);

    // ------------------------------------------------------------------
    // Precedence
    // ------------------------------------------------------------------

    /// Return the binding precedence for `head_name`.
    ///
    /// Higher values bind more tightly.  Unknown heads should return
    /// [`PREC_CALL`].
    fn precedence(&self, head_name: &str) -> u32;

    /// Return `true` if `head_name` is right-associative.
    ///
    /// The standard CAS right-associative operator is `"Pow"` (`a^b^c`
    /// parses as `a^(b^c)`).
    fn is_right_associative(&self, head_name: &str) -> bool;

    // ------------------------------------------------------------------
    // Surface sugar
    // ------------------------------------------------------------------

    /// Optionally rewrite `node` before the walker dispatches.
    ///
    /// Return a new `IRNode` to replace `node`, or `None` to fall through
    /// to normal dispatch.
    ///
    /// # Examples
    ///
    /// - `Add(x, Neg(y))` → `Sub(x, y)` (avoids double-negative)
    /// - `Mul(x, Inv(y))` → `Div(x, y)`
    /// - `Mul(-1, x)` → `Neg(x)`
    fn try_sugar(&self, node: &IRApply) -> Option<IRNode>;
}
