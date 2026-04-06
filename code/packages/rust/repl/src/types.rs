//! Core result type for REPL evaluations.
//!
//! # What is an evaluation result?
//!
//! Every time the user presses Enter in a REPL, their input goes through
//! an evaluation step. That step can have one of three outcomes:
//!
//! 1. **Success** — the expression was understood and evaluated. There may
//!    or may not be output to display (e.g., a statement like `x = 1` has no
//!    visible output, but `1 + 1` should display `2`).
//!
//! 2. **Error** — the expression was malformed or caused a runtime error.
//!    The error is *recoverable*: the REPL continues and shows the message.
//!    This is distinct from a hard crash (panic), which the runner catches
//!    separately with `std::panic::catch_unwind`.
//!
//! 3. **Quit** — the user or language signalled that the session should end.
//!    Typically triggered by `:quit`, `exit()`, `Ctrl-D`, etc.
//!
//! # Design rationale
//!
//! Using an enum rather than `Result<Option<String>, String>` makes the
//! quit state explicit at the type level. A language implementor cannot
//! accidentally conflate "error" with "session ended" — they must pick
//! a variant intentionally.

// ===========================================================================
// Mode — sync vs async evaluation
// ===========================================================================

/// Controls how the REPL runner dispatches `eval` calls.
///
/// # Variants
///
/// - [`Mode::Async`] (default) — each `eval` call is run on a dedicated OS
///   thread. The main thread drives the `Waiting` animation by polling the
///   result channel at `tick_ms` intervals. Best for interactive terminals
///   where a spinner or progress animation is desirable.
///
/// - [`Mode::Sync`] — `eval` is called directly on the calling thread via
///   `std::panic::catch_unwind`. No threads are spawned, no channels are
///   created, and the `waiting` argument to `run_with_options` is ignored
///   (and may be `None`). Best for scripted/test contexts where concurrency
///   adds no value.
///
/// # Examples
///
/// ```
/// use repl::types::Mode;
///
/// // The default
/// assert_eq!(Mode::default(), Mode::Async);
///
/// // Explicit sync
/// let m = Mode::Sync;
/// assert_eq!(m, Mode::Sync);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Eval runs on a dedicated OS thread; the `Waiting` animation ticks on
    /// the calling thread while waiting for the result.
    Async,

    /// Eval runs directly on the calling thread via `catch_unwind`. The
    /// `waiting` argument is unused and may be `None`.
    Sync,
}

impl Default for Mode {
    /// Returns [`Mode::Async`], preserving backwards compatibility with
    /// callers that use `run_with_io` (which always uses async mode).
    fn default() -> Self {
        Mode::Async
    }
}

/// The result of evaluating one line (or block) of user input.
///
/// Returned by [`Language::eval`](crate::language::Language::eval) and
/// threaded through the REPL runner to determine what to print and whether
/// to continue looping.
///
/// # Examples
///
/// ```
/// use repl::types::EvalResult;
///
/// // A successful evaluation with output
/// let r = EvalResult::Ok(Some("42".to_string()));
///
/// // A successful evaluation with no printable output
/// let r = EvalResult::Ok(None);
///
/// // A recoverable error
/// let r = EvalResult::Error("undefined variable: x".to_string());
///
/// // Session end signal
/// let r = EvalResult::Quit;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalResult {
    /// Evaluation succeeded.
    ///
    /// `Some(output)` — print this string to the user, then continue.
    /// `None` — nothing to display; just show the next prompt.
    Ok(Option<String>),

    /// A recoverable evaluation error.
    ///
    /// Display the message to the user (e.g., prefixed with `Error: `) and
    /// continue the session. The REPL does *not* exit on `Error`.
    Error(String),

    /// The session should end.
    ///
    /// The runner will stop reading input after receiving this variant.
    /// Typically triggered by the user typing `:quit`, `exit()`, or EOF.
    Quit,
}
