//! The REPL runner — the core Read-Eval-Print Loop.
//!
//! # How the loop works
//!
//! ```text
//!                 ┌─────────────────────────────────────────────┐
//!                 │                  MAIN THREAD                │
//!                 │                                             │
//!  input_fn() ───►│ read line                                   │
//!                 │     │                                       │
//!                 │     ▼                                       │
//!                 │  show prompt                                │
//!                 │     │                                       │
//!                 │     ▼                                       │
//!                 │  spawn eval thread ─────────────────────────┤──► WORKER THREAD
//!                 │     │                                       │        │
//!                 │     ▼                                       │        ▼
//!                 │  waiting.start()                            │    catch_unwind {
//!                 │     │                                       │        lang.eval(input)
//!                 │     ▼                                       │    }
//!  tick loop:     │  recv_timeout(tick_ms)                      │        │
//!  ┌──────────────┤     │            │                          │        ▼
//!  │  tick()      │     │ timeout    │ result                   │    tx.send(result)
//!  └──────────────┤     ▼            ▼                          │
//!                 │  waiting.tick() waiting.stop()              │
//!                 │                  │                          │
//!                 │                  ▼                          │
//!                 │              output_fn()                    │
//!                 │                  │                          │
//!                 │                  ▼                          │
//!                 │            Quit? ──yes──► return            │
//!                 │                  │                          │
//!                 │                  no                         │
//!                 │                  │                          │
//!                 └──────────────────┘                          │
//!                         back to top                           │
//!                 └─────────────────────────────────────────────┘
//! ```
//!
//! # I/O injection
//!
//! Both `run` and `run_with_io` accept closures for reading and writing.
//! This means the same loop logic can drive:
//!
//! - an interactive terminal (stdin / stdout)
//! - a test harness (in-memory Vec<String>)
//! - a network socket
//! - a file-based script runner
//!
//! # Panic isolation
//!
//! Every `eval` call is wrapped in `std::panic::catch_unwind`. A panicking
//! language backend surfaces as `EvalResult::Error("unexpected panic")` rather
//! than crashing the entire process. This is important for REPL longevity —
//! one bad expression should not kill the session.
//!
//! Note: `catch_unwind` only works for panics that unwind the stack. Panics
//! with `panic = "abort"` in the profile will still terminate the process.

use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use crate::language::Language;
use crate::prompt::Prompt;
use crate::types::{EvalResult, Mode};
use crate::waiting::Waiting;

// ===========================================================================
// run_with_io — the primary public API
// ===========================================================================

/// Run a REPL loop with fully injected I/O.
///
/// This is the most general entry point. It accepts closures for both input
/// and output, making it trivially testable without terminal I/O.
///
/// # Arguments
///
/// - `language` — the evaluation backend (wrapped in `Arc` for thread sharing)
/// - `prompt` — supplies the prompt strings
/// - `waiting` — controls the animation shown during long evals
/// - `input_fn` — called to read one line of input; returns `None` on EOF/quit
/// - `output_fn` — called to write one line of output (prompt strings, eval
///   results, error messages)
///
/// # Loop termination
///
/// The loop ends when:
/// 1. `input_fn` returns `None` (EOF / Ctrl-D).
/// 2. The language's `eval` returns `EvalResult::Quit`.
///
/// # Example (interactive terminal)
///
/// ```no_run
/// use std::sync::Arc;
/// use std::io::{self, BufRead, Write};
/// use repl::runner::run_with_io;
/// use repl::echo_language::EchoLanguage;
/// use repl::default_prompt::DefaultPrompt;
/// use repl::silent_waiting::SilentWaiting;
/// use repl::prompt::Prompt;
///
/// let lang    = Arc::new(EchoLanguage);
/// let prompt  = Arc::new(DefaultPrompt);
/// let waiting = Arc::new(SilentWaiting);
///
/// let stdin = io::stdin();
/// let mut lines = stdin.lock().lines();
///
/// run_with_io(
///     lang,
///     prompt.clone(),
///     waiting,
///     || {
///         print!("{}", DefaultPrompt.global_prompt());
///         io::stdout().flush().ok();
///         lines.next().and_then(|r| r.ok())
///     },
///     |s| println!("{s}"),
/// );
/// ```
///
/// # Example (test / scripted)
///
/// ```
/// use std::sync::Arc;
/// use repl::runner::run_with_io;
/// use repl::echo_language::EchoLanguage;
/// use repl::default_prompt::DefaultPrompt;
/// use repl::silent_waiting::SilentWaiting;
///
/// let inputs = vec!["hello".to_string(), ":quit".to_string()];
/// let mut iter = inputs.into_iter();
/// let mut outputs: Vec<String> = Vec::new();
///
/// run_with_io(
///     Arc::new(EchoLanguage),
///     Arc::new(DefaultPrompt),
///     Arc::new(SilentWaiting),
///     || iter.next(),
///     |s| outputs.push(s.to_string()),
/// );
///
/// assert_eq!(outputs, vec!["hello"]);
/// ```
/// Run a REPL loop with fully injected I/O.
///
/// This is a convenience wrapper around [`run_with_options`] that always uses
/// [`Mode::Async`] (goroutine + channel + waiting animation). All existing
/// callers continue to work without any changes.
///
/// # Arguments
///
/// - `language` — the evaluation backend (wrapped in `Arc` for thread sharing)
/// - `prompt` — supplies the prompt strings
/// - `waiting` — controls the animation shown during long evals
/// - `input_fn` — called to read one line of input; returns `None` on EOF/quit
/// - `output_fn` — called to write one line of output (prompt strings, eval
///   results, error messages)
///
/// # Loop termination
///
/// The loop ends when:
/// 1. `input_fn` returns `None` (EOF / Ctrl-D).
/// 2. The language's `eval` returns `EvalResult::Quit`.
///
/// # Example (interactive terminal)
///
/// ```no_run
/// use std::sync::Arc;
/// use std::io::{self, BufRead, Write};
/// use repl::runner::run_with_io;
/// use repl::echo_language::EchoLanguage;
/// use repl::default_prompt::DefaultPrompt;
/// use repl::silent_waiting::SilentWaiting;
/// use repl::prompt::Prompt;
///
/// let lang    = Arc::new(EchoLanguage);
/// let prompt  = Arc::new(DefaultPrompt);
/// let waiting = Arc::new(SilentWaiting);
///
/// let stdin = io::stdin();
/// let mut lines = stdin.lock().lines();
///
/// run_with_io(
///     lang,
///     prompt.clone(),
///     waiting,
///     || {
///         print!("{}", DefaultPrompt.global_prompt());
///         io::stdout().flush().ok();
///         lines.next().and_then(|r| r.ok())
///     },
///     |s| println!("{s}"),
/// );
/// ```
///
/// # Example (test / scripted)
///
/// ```
/// use std::sync::Arc;
/// use repl::runner::run_with_io;
/// use repl::echo_language::EchoLanguage;
/// use repl::default_prompt::DefaultPrompt;
/// use repl::silent_waiting::SilentWaiting;
///
/// let inputs = vec!["hello".to_string(), ":quit".to_string()];
/// let mut iter = inputs.into_iter();
/// let mut outputs: Vec<String> = Vec::new();
///
/// run_with_io(
///     Arc::new(EchoLanguage),
///     Arc::new(DefaultPrompt),
///     Arc::new(SilentWaiting),
///     || iter.next(),
///     |s| outputs.push(s.to_string()),
/// );
///
/// assert_eq!(outputs, vec!["hello"]);
/// ```
pub fn run_with_io<L, P, W, I, O>(
    language: Arc<L>,
    prompt: Arc<P>,
    waiting: Arc<W>,
    input_fn: I,
    output_fn: O,
) where
    L: Language + 'static,
    P: Prompt,
    W: Waiting + 'static,
    I: FnMut() -> Option<String>,
    O: FnMut(&str),
{
    run_with_options(language, prompt, Some(waiting), Mode::Async, input_fn, output_fn);
}

/// Run a REPL loop with explicit mode control and fully injected I/O.
///
/// This is the most general entry point. It accepts closures for both input
/// and output, a [`Mode`] selector, and an optional [`Waiting`] implementation.
///
/// # Arguments
///
/// - `language` — the evaluation backend (wrapped in `Arc` for thread sharing)
/// - `prompt` — supplies the prompt strings
/// - `waiting` — animation shown during long evals; only consulted in
///   [`Mode::Async`]. May be `None` when `mode == Mode::Sync`.
/// - `mode` — [`Mode::Async`] or [`Mode::Sync`] (see [`Mode`] for details)
/// - `input_fn` — called to read one line of input; returns `None` on EOF
/// - `output_fn` — called to write one line of output
///
/// # Sync vs Async
///
/// | | `Mode::Async` | `Mode::Sync` |
/// |---|---|---|
/// | Eval thread | dedicated OS thread | calling thread |
/// | Waiting animation | yes (via `waiting`) | skipped |
/// | `waiting` argument | must be `Some(…)` | ignored; `None` is fine |
/// | Panic handling | `catch_unwind` in worker thread | `catch_unwind` on caller |
///
/// # Loop termination
///
/// The loop ends when:
/// 1. `input_fn` returns `None` (EOF / Ctrl-D).
/// 2. The language's `eval` returns `EvalResult::Quit`.
///
/// # Example (sync mode, ideal for tests)
///
/// ```
/// use std::sync::Arc;
/// use repl::runner::run_with_options;
/// use repl::echo_language::EchoLanguage;
/// use repl::default_prompt::DefaultPrompt;
/// use repl::silent_waiting::SilentWaiting;
/// use repl::types::Mode;
///
/// let inputs = vec!["hello".to_string(), ":quit".to_string()];
/// let mut iter = inputs.into_iter();
/// let mut outputs: Vec<String> = Vec::new();
///
/// run_with_options(
///     Arc::new(EchoLanguage),
///     Arc::new(DefaultPrompt),
///     None::<Arc<SilentWaiting>>,   // waiting unused in sync mode
///     Mode::Sync,
///     || iter.next(),
///     |s| outputs.push(s.to_string()),
/// );
///
/// assert_eq!(outputs, vec!["hello"]);
/// ```
pub fn run_with_options<L, P, W, I, O>(
    language: Arc<L>,
    prompt: Arc<P>,
    waiting: Option<Arc<W>>,
    mode: Mode,
    mut input_fn: I,
    mut output_fn: O,
) where
    L: Language + 'static,
    P: Prompt,
    W: Waiting,
    I: FnMut() -> Option<String>,
    O: FnMut(&str),
{
    loop {
        // ------------------------------------------------------------------
        // 1. Read one line of input. EOF → stop the loop.
        // ------------------------------------------------------------------
        let input = match input_fn() {
            Some(line) => line,
            None => break,
        };

        // ------------------------------------------------------------------
        // 2. Evaluate — async or sync depending on `mode`.
        // ------------------------------------------------------------------
        let result = match mode {
            Mode::Sync => {
                // Sync mode: call eval directly on this thread.
                //
                // catch_unwind turns any panic into EvalResult::Error so the
                // REPL session survives a crashing evaluator — same guarantee
                // as async mode.
                let lang = Arc::clone(&language);
                let input_owned = input.clone();
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    lang.eval(&input_owned)
                }))
                .unwrap_or_else(|_| EvalResult::Error("unexpected panic".to_string()))
            }

            Mode::Async => {
                // Async mode: spawn a background thread to evaluate the input.
                //
                // We clone the Arc and the input string so the closure is
                // 'static and owns all its data. The result is sent back over
                // a one-shot mpsc channel.
                let (tx, rx) = std::sync::mpsc::channel::<EvalResult>();
                let lang = Arc::clone(&language);
                let input_owned = input.clone();

                std::thread::spawn(move || {
                    // AssertUnwindSafe is correct here: we own `lang`
                    // exclusively inside this thread via Arc, and `eval` is
                    // &self (shared ref). The caller guarantees
                    // Language: Send + Sync, so concurrent access is safe.
                    // We only need AssertUnwindSafe to satisfy catch_unwind's
                    // UnwindSafe bound.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        lang.eval(&input_owned)
                    }))
                    .unwrap_or_else(|_| EvalResult::Error("unexpected panic".to_string()));
                    tx.send(result).ok();
                });

                // Poll for the result, advancing the waiting animation on
                // each timeout tick.
                let w = waiting.as_ref().expect(
                    "run_with_options: waiting must be Some(_) when mode == Mode::Async",
                );
                let mut state = w.start();

                loop {
                    match rx.recv_timeout(Duration::from_millis(w.tick_ms())) {
                        Ok(result) => {
                            w.stop(state);
                            break result;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            state = w.tick(state);
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            // The sender was dropped without sending — treat
                            // as an internal error and continue the loop.
                            w.stop(state);
                            break EvalResult::Error("evaluator disconnected".to_string());
                        }
                    }
                }
            }
        };

        // ------------------------------------------------------------------
        // 3. Handle the result:
        //    - Ok(Some(s)) → print the output
        //    - Ok(None)    → nothing to print
        //    - Error(msg)  → print the error
        //    - Quit        → stop the loop
        // ------------------------------------------------------------------
        match result {
            EvalResult::Ok(Some(output)) => output_fn(&output),
            EvalResult::Ok(None) => {}
            EvalResult::Error(msg) => output_fn(&format!("Error: {msg}")),
            EvalResult::Quit => break,
        }

        // Suppress unused variable warning — prompt is intentionally held
        // in scope so callers can use it inside `input_fn` closures.
        let _ = &prompt;
    }
}

// ===========================================================================
// run — convenience wrapper using stdin/stdout
// ===========================================================================

/// Run a REPL loop reading from `stdin` and writing to `stdout`.
///
/// This is a thin convenience wrapper around [`run_with_io`] that wires up
/// standard I/O. For production use you may want to add readline history,
/// tab completion, etc. — in that case, call `run_with_io` directly and
/// supply your own input closure.
///
/// The prompt strings from `prompt` are written to stdout before each
/// `stdin` read.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use repl::runner::run;
/// use repl::echo_language::EchoLanguage;
/// use repl::default_prompt::DefaultPrompt;
/// use repl::silent_waiting::SilentWaiting;
///
/// run(
///     Arc::new(EchoLanguage),
///     Arc::new(DefaultPrompt),
///     Arc::new(SilentWaiting),
/// );
/// ```
pub fn run<L, P, W>(language: Arc<L>, prompt: Arc<P>, waiting: Arc<W>)
where
    L: Language + 'static,
    P: Prompt,
    W: Waiting + 'static,
{
    use std::io::{self, BufRead, Write};

    let stdin = io::stdin();
    let stdout = io::stdout();

    // We need the prompt Arc inside the closure, so clone it.
    let prompt_for_input = Arc::clone(&prompt);

    // Collect lines from stdin lazily.
    let mut lines = stdin.lock().lines();

    run_with_io(
        language,
        prompt,
        waiting,
        move || {
            // Print the prompt before reading. Flush immediately so the
            // user sees it before blocking on stdin.
            {
                let mut out = stdout.lock();
                write!(out, "{}", prompt_for_input.global_prompt()).ok();
                out.flush().ok();
            }
            lines.next().and_then(|r| r.ok())
        },
        |s| println!("{s}"),
    );
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_prompt::DefaultPrompt;
    use crate::echo_language::EchoLanguage;
    use crate::silent_waiting::SilentWaiting;

    fn run_script(inputs: Vec<&str>) -> Vec<String> {
        let inputs: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
        let mut iter = inputs.into_iter();
        let mut outputs: Vec<String> = Vec::new();

        run_with_io(
            Arc::new(EchoLanguage),
            Arc::new(DefaultPrompt),
            Arc::new(SilentWaiting),
            || iter.next(),
            |s| outputs.push(s.to_string()),
        );
        outputs
    }

    #[test]
    fn test_echo_single_line() {
        let out = run_script(vec!["hello", ":quit"]);
        assert_eq!(out, vec!["hello"]);
    }

    #[test]
    fn test_quit_produces_no_output() {
        let out = run_script(vec![":quit"]);
        assert!(out.is_empty());
    }

    #[test]
    fn test_eof_stops_loop() {
        // No `:quit` — EOF (empty iterator) stops the loop.
        let out = run_script(vec!["a", "b"]);
        assert_eq!(out, vec!["a", "b"]);
    }

    #[test]
    fn test_multiple_lines_echoed() {
        let out = run_script(vec!["one", "two", "three", ":quit"]);
        assert_eq!(out, vec!["one", "two", "three"]);
    }
}
