//! Dialect-agnostic IR walker.
//!
//! The walker descends an [`IRNode`] tree and emits source text.  It
//! tracks one piece of state — `min_prec`, the minimum precedence the
//! emitted text must have to avoid being parenthesized by its parent.
//! Every dialect hook (operator spellings, brackets, sugar) is consulted
//! via the [`Dialect`] trait; the walker itself does not know which
//! language it is printing.
//!
//! # Algorithm
//!
//! For an `IRApply(head, args)` whose head is a known operator:
//!
//! 1. Try `dialect.try_sugar(node)`; if it returns a rewritten tree,
//!    format that recursively.
//! 2. Try a registered head formatter (see [`register_head_formatter`]).
//! 3. If `head` is `"List"`, emit the list brackets.
//! 4. If `head` is a unary operator with arity 1, emit `op arg` with
//!    `arg` formatted at the parent precedence level.
//! 5. If `head` is a binary operator with arity ≥ 2, emit
//!    `arg op arg op …` with each child formatted at the correct
//!    `min_prec`.  Left-associative operators give non-first children
//!    `parent_prec + 1`; right-associative operators give non-last
//!    children `parent_prec + 1`.
//! 6. Otherwise fall back to function-call form: `name(arg, arg, …)`.
//!
//! After steps 4 or 5, parentheses are wrapped iff the operator's
//! precedence is strictly less than `min_prec`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use symbolic_ir::{IRApply, IRNode};

use crate::dialect::{Dialect, PREC_ATOM};

// ---------------------------------------------------------------------------
// Head-formatter registry
// ---------------------------------------------------------------------------

/// A function registered for a custom head.
///
/// Receives:
/// - the `IRApply` node,
/// - the active [`Dialect`] (for re-using dialect methods),
/// - a `fmt` closure that formats child nodes with no precedence context.
///
/// Must return the formatted `String`.
///
/// # Example
///
/// ```rust
/// use cas_pretty_printer::register_head_formatter;
/// use symbolic_ir::IRNode;
///
/// register_head_formatter("Matrix", |node, _dialect, fmt| {
///     let rows: Vec<String> = node.args.iter()
///         .map(|row| {
///             if let IRNode::Apply(a) = row {
///                 let cells: Vec<String> = a.args.iter().map(|c| fmt(c)).collect();
///                 format!("[{}]", cells.join(", "))
///             } else {
///                 fmt(row)
///             }
///         })
///         .collect();
///     format!("matrix({})", rows.join(", "))
/// });
/// ```
pub type HeadFormatterFn = dyn Fn(&IRApply, &dyn Dialect, &dyn Fn(&IRNode) -> String) -> String
    + Send
    + Sync;

// The registry stores Arc<HeadFormatterFn> so we can clone the Arc out of
// the map before calling the formatter, avoiding a deadlock when the
// formatter calls back into pretty() for child nodes.
static HEAD_FORMATTERS: OnceLock<Mutex<HashMap<String, Arc<HeadFormatterFn>>>> = OnceLock::new();

fn head_formatters() -> &'static Mutex<HashMap<String, Arc<HeadFormatterFn>>> {
    HEAD_FORMATTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a custom head formatter for `head_name`.
///
/// The formatter is called whenever the walker encounters an `IRApply`
/// whose head is a symbol with the given name.  It receives the node,
/// the active dialect, and a `fmt(child)` helper that formats children
/// with no precedence context; the formatter is responsible for any
/// nesting logic.  It must return a `String`.
///
/// # Thread safety
///
/// The registry is protected by a global `Mutex`.  Formatters must be
/// `Send + Sync + 'static`.
pub fn register_head_formatter<F>(head_name: &str, formatter: F)
where
    F: Fn(&IRApply, &dyn Dialect, &dyn Fn(&IRNode) -> String) -> String
        + Send
        + Sync
        + 'static,
{
    head_formatters()
        .lock()
        .unwrap()
        .insert(head_name.to_string(), Arc::new(formatter));
}

/// Remove the formatter registered for `head_name`, if any.
///
/// After this call the head falls back to function-call form.
/// Mostly used in tests to restore a clean state.
pub fn unregister_head_formatter(head_name: &str) {
    head_formatters().lock().unwrap().remove(head_name);
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Format `node` as source text in the given `dialect`.
///
/// Currently only linear (single-line) output is produced.  A `"2D"`
/// ASCII layout mode may be added in a future version.
///
/// # Panics
///
/// Panics if an unknown `IRNode` variant is encountered (should never
/// happen with well-formed nodes).
///
/// # Example
///
/// ```rust
/// use cas_pretty_printer::{pretty, MacsymaDialect};
/// use symbolic_ir::{apply, int, sym, ADD, POW};
///
/// let x = sym("x");
/// let expr = apply(sym(ADD), vec![
///     apply(sym(POW), vec![x.clone(), int(2)]),
///     int(1),
/// ]);
/// assert_eq!(pretty(&expr, &MacsymaDialect), "x^2 + 1");
/// ```
pub fn pretty(node: &IRNode, dialect: &dyn Dialect) -> String {
    format_node(node, dialect, 0)
}

// ---------------------------------------------------------------------------
// Recursive formatting helpers
// ---------------------------------------------------------------------------

/// Core recursive formatter.  `min_prec` is the minimum precedence the
/// result must have to avoid being wrapped in parentheses by the caller.
pub(crate) fn format_node(node: &IRNode, dialect: &dyn Dialect, min_prec: u32) -> String {
    match node {
        IRNode::Integer(v) => format_integer(*v, dialect, min_prec),
        IRNode::Rational(n, d) => format_rational(*n, *d, dialect, min_prec),
        IRNode::Float(v) => format_float(*v, dialect, min_prec),
        IRNode::Str(s) => dialect.format_string(s),
        IRNode::Symbol(name) => dialect.format_symbol(name),
        IRNode::Apply(a) => format_apply(a, dialect, min_prec),
    }
}

/// Format an `Integer` node.
///
/// Negative literals need parens when they appear inside a tighter-binding
/// operator context.  Example: `2^(-3)` — without parens the caret would
/// bind to just `3`, not `-3`.
fn format_integer(value: i64, dialect: &dyn Dialect, min_prec: u32) -> String {
    let text = dialect.format_integer(value);
    if value < 0 && min_prec > 0 {
        format!("({})", text)
    } else {
        text
    }
}

/// Format a `Rational` node.
///
/// Rationals always contain a `/` in their textual form; negative
/// numerators add a leading minus.  Both cases need parenthesisation in
/// non-zero precedence contexts.
fn format_rational(numer: i64, denom: i64, dialect: &dyn Dialect, min_prec: u32) -> String {
    let text = dialect.format_rational(numer, denom);
    // Wrap if: numerator negative OR text contains "/" (i.e. not reduced
    // to an integer), AND we are inside a tighter-binding context.
    if (numer < 0 || text.contains('/')) && min_prec > 0 {
        format!("({})", text)
    } else {
        text
    }
}

/// Format a `Float` node.
///
/// Negative floats need parens in the same situations as negative integers.
fn format_float(value: f64, dialect: &dyn Dialect, min_prec: u32) -> String {
    let text = dialect.format_float(value);
    if value < 0.0 && min_prec > 0 {
        format!("({})", text)
    } else {
        text
    }
}

/// Format an `IRApply` node.
///
/// This is the heart of the walker — it implements the 6-step dispatch
/// described in the module doc-comment.
fn format_apply(node: &IRApply, dialect: &dyn Dialect, min_prec: u32) -> String {
    // ---- Step 1: Sugar -------------------------------------------------------
    // Ask the dialect if it wants to rewrite this node.  If so, format the
    // rewritten node recursively.  The recursive call may trigger further sugar.
    if let Some(sugared) = dialect.try_sugar(node) {
        return format_node(&sugared, dialect, min_prec);
    }

    // Extract the head name (if the head is a Symbol — it usually is).
    let head_name: Option<&str> = if let IRNode::Symbol(s) = &node.head {
        Some(s.as_str())
    } else {
        None
    };

    // ---- Step 2: Custom head formatter -------------------------------------
    // Clone the Arc out of the registry before calling, so we do not hold
    // the Mutex across the user's callback (which may call pretty() itself,
    // causing a deadlock if we held the lock).
    if let Some(name) = head_name {
        let maybe_fmt: Option<Arc<HeadFormatterFn>> = head_formatters()
            .lock()
            .unwrap()
            .get(name)
            .cloned();
        if let Some(formatter) = maybe_fmt {
            return formatter(node, dialect, &|child| format_node(child, dialect, 0));
        }
    }

    // ---- Step 3: List literal -----------------------------------------------
    if head_name == Some("List") {
        return format_list(node, dialect);
    }

    // ---- Step 4: Unary op ---------------------------------------------------
    // Only triggered for nodes with exactly 1 argument.
    if let Some(name) = head_name {
        if node.args.len() == 1 {
            if let Some(op_text) = dialect.unary_op(name) {
                let prec = dialect.precedence(name);
                let inner = format_node(&node.args[0], dialect, prec);
                let text = format!("{}{}", op_text, inner);
                return wrap_if_needed(text, prec, min_prec);
            }
        }
    }

    // ---- Step 5: Binary / n-ary op ------------------------------------------
    // Triggered for nodes with 2 or more arguments when the head has an
    // infix spelling.
    if let Some(name) = head_name {
        if node.args.len() >= 2 {
            if let Some(op_text) = dialect.binary_op(name) {
                let prec = dialect.precedence(name);
                let right_assoc = dialect.is_right_associative(name);
                let n = node.args.len();

                // Associativity and parenthesisation:
                //
                //  Left-assoc (e.g. Sub, Add):
                //    - i > 0  (non-first args) → child_prec = prec + 1
                //    This ensures `a - (b - c)` gets parens; `a - b - c`
                //    (left parse) does not.
                //
                //  Right-assoc (e.g. Pow):
                //    - i < n-1 (non-last args) → child_prec = prec + 1
                //    This ensures `(a^b)^c` gets parens; `a^(b^c)`
                //    (right parse) does not.
                let parts: Vec<String> = node
                    .args
                    .iter()
                    .enumerate()
                    .map(|(i, arg)| {
                        let child_prec =
                            if (right_assoc && i < n - 1) || (!right_assoc && i > 0) {
                                prec + 1
                            } else {
                                prec
                            };
                        format_node(arg, dialect, child_prec)
                    })
                    .collect();

                let text = parts.join(&op_text);
                return wrap_if_needed(text, prec, min_prec);
            }
        }
    }

    // ---- Step 6: Function-call form -----------------------------------------
    format_call(node, dialect)
}

/// Emit a list literal: `[a, b, c]` (or `{a, b, c}` in Mathematica).
fn format_list(node: &IRApply, dialect: &dyn Dialect) -> String {
    let (open, close) = dialect.list_brackets();
    let args = node
        .args
        .iter()
        .map(|a| format_node(a, dialect, 0))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}{}{}", open, args, close)
}

/// Emit a function-call expression: `name(arg, arg, …)`.
///
/// If the head is not a plain symbol (higher-order call), format the head
/// itself at atom precedence so it gets wrapped if needed.
fn format_call(node: &IRApply, dialect: &dyn Dialect) -> String {
    let name = match &node.head {
        IRNode::Symbol(s) => dialect.function_name(s),
        other => format_node(other, dialect, PREC_ATOM),
    };
    let (open, close) = dialect.call_brackets();
    let args = node
        .args
        .iter()
        .map(|a| format_node(a, dialect, 0))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}{}{}{}", name, open, args, close)
}

/// Wrap `text` in parentheses iff the node's own precedence `prec` is
/// strictly less than the surrounding context's minimum `min_prec`.
///
/// A node with `prec < min_prec` would be misread by the parser without
/// explicit grouping.  A node with `prec >= min_prec` is already tight
/// enough on its own.
fn wrap_if_needed(text: String, prec: u32, min_prec: u32) -> String {
    if prec < min_prec {
        format!("({})", text)
    } else {
        text
    }
}
