//! Error types for the Twig base type checker (TW05-B).
//!
//! ## Two classes of errors
//!
//! TW05-B draws a sharp line between two kinds of bad news:
//!
//! 1. **Parse failures** (`TwigTypeCheckError::Parse`) — the source text
//!    couldn't be lexed or parsed at all.  These propagate as the
//!    `Err` side of `Result` so callers that want to bail-early can match on
//!    them without opening the `TypeCheckResult`.
//!
//! 2. **Type errors** (`TypeErrorDiagnostic`) — the source parsed fine but
//!    the type checker found a violation (unresolved variable, arity mismatch,
//!    non-exhaustive match, …).  These are *not* fatal from the checker's
//!    perspective — the checker continues, collects every error it can find,
//!    and returns them all inside `TypeCheckResult::errors`.  Whether to
//!    treat them as blocking is the caller's decision (Strict vs Lenient mode).
//!
//! ## Why separate them?
//!
//! A pipeline that feeds type-checked output into a subsequent stage (like
//! the IIR compiler) needs to know upfront whether *any* typed AST was
//! produced.  If parsing fails, there is no AST — returning `Err` is the
//! right signal.  If type checking has warnings, there *is* an AST — the
//! caller can decide whether to use it.

use twig_parser::TwigParseError;

/// Fatal error surfaced by [`crate::type_check`].
///
/// A `TwigTypeCheckError` is only returned when the source is so broken
/// that no AST could be produced.  Type errors discovered during the
/// type-checking walk are not placed here — they live in
/// [`type_checker_protocol::TypeCheckResult::errors`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TwigTypeCheckError {
    /// The source text failed to lex or parse.
    ///
    /// The embedded [`TwigParseError`] carries the exact source position and a
    /// human-readable message.
    Parse(TwigParseError),
}

impl std::fmt::Display for TwigTypeCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwigTypeCheckError::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

impl std::error::Error for TwigTypeCheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TwigTypeCheckError::Parse(e) => Some(e),
        }
    }
}

impl From<TwigParseError> for TwigTypeCheckError {
    fn from(e: TwigParseError) -> Self {
        TwigTypeCheckError::Parse(e)
    }
}
