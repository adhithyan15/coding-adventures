//! Call arity checking for `(fn arg0 arg1 …)` expressions.
//!
//! ## Why arity matters
//!
//! Twig is a Lisp: function application is written `(fn arg …)`.  The parser
//! accepts any number of arguments to any function — a call like `(f 1 2 3)`
//! where `f` was declared `(define (f x) x)` is syntactically valid but
//! semantically wrong.  At runtime the VM would raise an arity error.
//!
//! TW05-B surfaces these mistakes at compile time, before IIR is even
//! emitted, so the developer gets a clear message ("expected 1 argument, got
//! 3") rather than a cryptic runtime crash.
//!
//! ## When does arity checking fire?
//!
//! Only when the function position of an `Apply` node resolves to a
//! `TwigKind::Function { arity }`.  If the function position is
//! `TwigKind::Any` (e.g. an unannotated higher-order parameter) the checker
//! silently skips — it can't know the expected arity.
//!
//! This means false negatives are possible, but never false positives.  A
//! warning is never emitted for a call that is actually valid.
//!
//! ## Error message format
//!
//! ```text
//! arity error: `factorial` expects 1 argument, got 2
//! arity error: anonymous function expects 3 arguments, got 0
//! ```

use type_checker_protocol::TypeErrorDiagnostic;

/// Emit an arity-error diagnostic if `expected != actual`.
///
/// `fn_name` is the display name shown in the error message — `Some("f")`
/// for a named function call, `None` for an anonymous lambda call.
///
/// `line` and `column` come from the `Apply` node's source position so the
/// editor can underline the opening parenthesis of the call site.
///
/// # Behaviour
///
/// - If `expected == actual` → no diagnostic is appended (happy path).
/// - If `expected != actual` → one `TypeErrorDiagnostic` is pushed onto
///   `errors`.  The caller is responsible for deciding whether to treat this
///   as a blocking error (Strict mode) or a warning (Lenient mode).
pub fn check_arity(
    fn_name: Option<&str>,
    expected: usize,
    actual: usize,
    line: usize,
    column: usize,
    errors: &mut Vec<TypeErrorDiagnostic>,
) {
    if expected == actual {
        return;
    }

    // Build a human-friendly label: named function or "anonymous function".
    let label = match fn_name {
        Some(name) => format!("`{name}`"),
        None => "anonymous function".to_owned(),
    };

    // English-correct pluralisation: "1 argument" vs "2 arguments".
    let arg_word = |n: usize| if n == 1 { "argument" } else { "arguments" };

    errors.push(TypeErrorDiagnostic {
        message: format!(
            "arity error: {label} expects {} {}, got {} {}",
            expected,
            arg_word(expected),
            actual,
            arg_word(actual),
        ),
        line,
        column,
    });
}
