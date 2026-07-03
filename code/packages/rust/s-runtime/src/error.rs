//! Errors raised while evaluating S code.
//!
//! S itself reports errors with short messages like `object 'x' not found` or
//! `non-numeric argument to binary operator`. We mirror that style but keep the
//! errors *typed* — the category is part of the runtime's public interface and
//! is easier to test against than a free-form string.

use std::fmt;

/// A runtime error from evaluating S.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SError {
    /// A lexical or syntax error from the front end.
    Parse(String),
    /// A name was used as a value but is not bound: `object 'x' not found`.
    Undefined(String),
    /// A value was called as if it were a function but is not callable.
    NotCallable(String),
    /// A built-in or call received arguments it cannot use.
    BadArgs(String),
    /// An operation was applied to the wrong type (e.g. arithmetic on strings).
    TypeError(String),
    /// An index was out of range or otherwise invalid.
    Index(String),
    /// A control-flow value (truth test) was missing/`NA`.
    Missing(String),
    /// A domain/precondition error surfaced from `statistics-core`.
    Domain(String),
    /// `break` / `next` used outside a loop, or any other control misuse.
    Control(String),
    /// An error raised explicitly by the program via `stop(...)`. Kept distinct
    /// from the type/index/etc. categories so a user-level `stop` is
    /// recognizable, but — like every other non-control error — it is catchable
    /// by `tryCatch(..., error = ...)`. The string is the already-concatenated
    /// message (the arguments of `stop`, coerced to character and joined).
    User(String),

    /// Internal control signal raised by `break` and caught by the enclosing
    /// loop. It only reaches a user as an error when `break` is used with no
    /// loop around it.
    Break,
    /// Internal control signal raised by `next` (S's `continue`).
    Next,
}

impl fmt::Display for SError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SError::Parse(m) => write!(f, "{m}"),
            SError::Undefined(name) => write!(f, "object '{name}' not found"),
            SError::NotCallable(m) => write!(f, "attempt to apply non-function: {m}"),
            SError::BadArgs(m) => write!(f, "invalid arguments: {m}"),
            SError::TypeError(m) => write!(f, "{m}"),
            SError::Index(m) => write!(f, "invalid subscript: {m}"),
            SError::Missing(m) => write!(f, "{m}"),
            SError::Domain(m) => write!(f, "{m}"),
            SError::Control(m) => write!(f, "{m}"),
            SError::User(m) => write!(f, "{m}"),
            SError::Break => write!(f, "no loop for break/next, jumping to top level"),
            SError::Next => write!(f, "no loop for break/next, jumping to top level"),
        }
    }
}

impl SError {
    /// Whether this error is a *catchable* condition for `tryCatch(error = ...)`.
    /// Every ordinary error qualifies; the internal [`SError::Break`] /
    /// [`SError::Next`] control signals do **not** — they are loop control, not
    /// program errors, and must propagate untouched to their enclosing loop.
    pub fn is_catchable(&self) -> bool {
        !matches!(self, SError::Break | SError::Next)
    }

    /// The human-readable condition message for a catchable error — exactly what
    /// `conditionMessage(e)` / `e$message` should report. This is the `Display`
    /// text, so a `stop("boom")` yields `"boom"` and a type error yields its
    /// message verbatim.
    pub fn condition_message(&self) -> String {
        self.to_string()
    }
}

impl std::error::Error for SError {}

/// Convenience alias for results produced by the evaluator.
pub type SResult<T> = Result<T, SError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_match_s_style() {
        assert_eq!(SError::Parse("oops".into()).to_string(), "oops");
        assert_eq!(
            SError::Undefined("x".into()).to_string(),
            "object 'x' not found"
        );
        assert_eq!(
            SError::NotCallable("double".into()).to_string(),
            "attempt to apply non-function: double"
        );
        assert_eq!(
            SError::BadArgs("bad".into()).to_string(),
            "invalid arguments: bad"
        );
        assert_eq!(SError::TypeError("te".into()).to_string(), "te");
        assert_eq!(
            SError::Index("oob".into()).to_string(),
            "invalid subscript: oob"
        );
        assert_eq!(SError::Missing("m".into()).to_string(), "m");
        assert_eq!(SError::Domain("d".into()).to_string(), "d");
        assert_eq!(SError::Control("c".into()).to_string(), "c");
        assert_eq!(SError::User("boom".into()).to_string(), "boom");
        assert!(SError::Break.to_string().contains("no loop"));
        assert!(SError::Next.to_string().contains("no loop"));
    }

    #[test]
    fn catchability_excludes_only_control_signals() {
        // Ordinary errors are catchable by tryCatch.
        assert!(SError::User("boom".into()).is_catchable());
        assert!(SError::TypeError("te".into()).is_catchable());
        assert!(SError::Undefined("x".into()).is_catchable());
        // The loop-control signals are not.
        assert!(!SError::Break.is_catchable());
        assert!(!SError::Next.is_catchable());
    }

    #[test]
    fn condition_message_is_the_display_text() {
        assert_eq!(SError::User("boom".into()).condition_message(), "boom");
        assert_eq!(
            SError::Undefined("x".into()).condition_message(),
            "object 'x' not found"
        );
    }

    #[test]
    fn is_a_std_error() {
        let e: &dyn std::error::Error = &SError::Undefined("z".into());
        assert_eq!(e.to_string(), "object 'z' not found");
    }
}
