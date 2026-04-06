//! The `Language` trait — pluggable evaluation backend.
//!
//! # What is a language backend?
//!
//! A REPL (Read-Eval-Print Loop) is language-agnostic at its core. The loop
//! itself only knows how to:
//!
//! - read a line of input,
//! - hand it to *something* for evaluation,
//! - display the result.
//!
//! The "something" is the [`Language`] trait. By swapping out the `Language`
//! implementation you can build REPLs for Lisp, Python, a calculator, a
//! command shell, a database query interface — anything that maps strings
//! to [`EvalResult`](crate::types::EvalResult).
//!
//! # Thread safety requirements
//!
//! The runner spawns eval work on a background thread (to allow a waiting
//! spinner while the user's code runs). Therefore the language backend must
//! be `Send + Sync` so it can be safely shared across thread boundaries via
//! `Arc<L>`.
//!
//! # Implementing `Language`
//!
//! ```rust
//! use repl::language::Language;
//! use repl::types::EvalResult;
//!
//! struct Calculator;
//!
//! impl Language for Calculator {
//!     fn eval(&self, input: &str) -> EvalResult {
//!         let trimmed = input.trim();
//!         if trimmed == ":quit" {
//!             return EvalResult::Quit;
//!         }
//!         // Try to parse and evaluate a simple integer expression
//!         match trimmed.parse::<i64>() {
//!             Ok(n) => EvalResult::Ok(Some(n.to_string())),
//!             Err(_) => EvalResult::Error(format!("cannot parse: {trimmed}")),
//!         }
//!     }
//! }
//! ```

use crate::types::EvalResult;

/// Pluggable evaluation backend for a REPL session.
///
/// Implement this trait to create a new language or evaluation environment.
/// The runner calls [`eval`](Language::eval) on a background thread, so your
/// implementation must be `Send + Sync`. Wrap expensive state in `Arc<Mutex<_>>`
/// if you need mutable access.
///
/// # Panic safety
///
/// The runner wraps every call to `eval` in `std::panic::catch_unwind`. If
/// your evaluator panics, the REPL surfaces it as
/// `EvalResult::Error("unexpected panic")` instead of crashing the process.
/// However, panics inside FFI or `extern "C"` code may still be unrecoverable.
pub trait Language: Send + Sync {
    /// Evaluate one line (or block) of user input.
    ///
    /// # Arguments
    ///
    /// - `input` — the raw string the user typed, including any trailing
    ///   whitespace (the runner does not strip it; that is the language's
    ///   prerogative).
    ///
    /// # Return value
    ///
    /// See [`EvalResult`](crate::types::EvalResult) for the three possible
    /// outcomes.
    fn eval(&self, input: &str) -> EvalResult;
}
