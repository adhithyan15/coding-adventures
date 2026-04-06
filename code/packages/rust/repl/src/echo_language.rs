//! `EchoLanguage` — a built-in language backend that echoes input back.
//!
//! # Purpose
//!
//! `EchoLanguage` is the simplest possible [`Language`](crate::language::Language)
//! implementation. It exists for three reasons:
//!
//! 1. **Testing** — integration tests can drive the REPL without a real
//!    evaluator, verifying I/O plumbing, quit handling, and error display.
//!
//! 2. **Demonstration** — new contributors can read this struct to understand
//!    the minimal contract required to implement a language backend.
//!
//! 3. **Scaffolding** — you can start a new language REPL by copying this
//!    struct and extending the `eval` method.
//!
//! # Behaviour
//!
//! | Input | Output |
//! |-------|--------|
//! | `:quit` | `EvalResult::Quit` — ends the session |
//! | anything else | `EvalResult::Ok(Some(input.to_string()))` — echo |
//!
//! Whitespace is *not* stripped; whatever the user typed (spaces included)
//! is echoed back verbatim, except for the `:quit` sentinel which is matched
//! after trimming.

use crate::language::Language;
use crate::types::EvalResult;

/// A language backend that echoes every line of input back to the user.
///
/// Recognises `:quit` (after trimming whitespace) as a session-end signal.
/// All other input — including empty strings — is returned as `Ok(Some(...))`.
///
/// # Examples
///
/// ```
/// use repl::echo_language::EchoLanguage;
/// use repl::language::Language;
/// use repl::types::EvalResult;
///
/// let lang = EchoLanguage;
///
/// assert_eq!(lang.eval("hello"), EvalResult::Ok(Some("hello".to_string())));
/// assert_eq!(lang.eval(":quit"), EvalResult::Quit);
/// assert_eq!(lang.eval("  :quit  "), EvalResult::Quit);
/// assert_eq!(lang.eval(""), EvalResult::Ok(Some("".to_string())));
/// ```
pub struct EchoLanguage;

impl Language for EchoLanguage {
    /// Evaluate a single line of input.
    ///
    /// - `:quit` (trimmed) → `EvalResult::Quit`
    /// - anything else → `EvalResult::Ok(Some(input.to_string()))`
    fn eval(&self, input: &str) -> EvalResult {
        // The `:quit` sentinel is matched after stripping surrounding
        // whitespace so that `  :quit  ` also ends the session. This mirrors
        // what most REPLs do with special commands.
        if input.trim() == ":quit" {
            return EvalResult::Quit;
        }
        EvalResult::Ok(Some(input.to_string()))
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_simple() {
        let lang = EchoLanguage;
        assert_eq!(lang.eval("hello"), EvalResult::Ok(Some("hello".to_string())));
    }

    #[test]
    fn test_echo_quit() {
        let lang = EchoLanguage;
        assert_eq!(lang.eval(":quit"), EvalResult::Quit);
    }

    #[test]
    fn test_echo_quit_with_whitespace() {
        let lang = EchoLanguage;
        assert_eq!(lang.eval("  :quit  "), EvalResult::Quit);
        assert_eq!(lang.eval("\t:quit\n"), EvalResult::Quit);
    }

    #[test]
    fn test_echo_empty_string() {
        let lang = EchoLanguage;
        assert_eq!(lang.eval(""), EvalResult::Ok(Some("".to_string())));
    }

    #[test]
    fn test_echo_preserves_content() {
        let lang = EchoLanguage;
        let input = "  spaces around  ";
        // Whitespace is preserved in the echo (not trimmed)
        assert_eq!(lang.eval(input), EvalResult::Ok(Some(input.to_string())));
    }
}
