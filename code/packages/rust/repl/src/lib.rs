//! # repl — a pluggable Read-Eval-Print Loop framework
//!
//! This crate provides the scaffolding for building interactive REPLs in Rust.
//! It is intentionally minimal: no external dependencies, standard library only.
//!
//! ## Architecture
//!
//! A REPL has four concerns, each captured by a trait or module:
//!
//! | Concern | Trait / module | Built-in implementation |
//! |---------|---------------|------------------------|
//! | Evaluating input | [`Language`] | [`EchoLanguage`] |
//! | Showing prompts | [`Prompt`] | [`DefaultPrompt`] |
//! | Animation while waiting | [`Waiting`] | [`SilentWaiting`] |
//! | The loop itself | [`runner`] | `run` / `run_with_io` |
//!
//! ## Minimal working example
//!
//! ```
//! use std::sync::Arc;
//! use repl::runner::run_with_io;
//! use repl::echo_language::EchoLanguage;
//! use repl::default_prompt::DefaultPrompt;
//! use repl::silent_waiting::SilentWaiting;
//!
//! let inputs = vec!["hello".to_string(), ":quit".to_string()];
//! let mut iter = inputs.into_iter();
//! let mut outputs = Vec::new();
//!
//! run_with_io(
//!     Arc::new(EchoLanguage),
//!     Arc::new(DefaultPrompt),
//!     Arc::new(SilentWaiting),
//!     || iter.next(),
//!     |s| outputs.push(s.to_string()),
//! );
//!
//! assert_eq!(outputs, vec!["hello"]);
//! ```
//!
//! ## Building your own language backend
//!
//! Implement the [`Language`] trait on your own struct:
//!
//! ```rust
//! use repl::language::Language;
//! use repl::types::EvalResult;
//!
//! struct MyLanguage;
//!
//! impl Language for MyLanguage {
//!     fn eval(&self, input: &str) -> EvalResult {
//!         match input.trim() {
//!             ":quit" | "exit" => EvalResult::Quit,
//!             expr => {
//!                 // … your evaluation logic here …
//!                 EvalResult::Ok(Some(format!("=> {expr}")))
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! Then call [`runner::run_with_io`] or [`runner::run`] with `Arc::new(MyLanguage)`.
//!
//! ## Thread model
//!
//! Each evaluation runs on a dedicated OS thread. The main thread drives the
//! waiting animation by polling the result channel with a timeout equal to
//! [`Waiting::tick_ms`](waiting::Waiting::tick_ms). A panic inside `eval` is
//! caught with `std::panic::catch_unwind` and surfaced as
//! `EvalResult::Error("unexpected panic")`.

// ---------------------------------------------------------------------------
// Module declarations
// ---------------------------------------------------------------------------

/// The [`EvalResult`](types::EvalResult) enum — the three possible outcomes
/// of evaluating one line of input.
pub mod types;

/// The [`Language`](language::Language) trait — pluggable evaluation backend.
pub mod language;

/// The [`Prompt`](prompt::Prompt) trait — customisable input prompts.
pub mod prompt;

/// The [`Waiting`](waiting::Waiting) trait — animated feedback during long
/// evaluations.
pub mod waiting;

/// The REPL runner — `run` and `run_with_io`.
pub mod runner;

/// Built-in [`Language`](language::Language) that echoes input back.
pub mod echo_language;

/// Built-in [`Prompt`](prompt::Prompt) using `"> "` and `"... "`.
pub mod default_prompt;

/// Built-in [`Waiting`](waiting::Waiting) that performs no I/O (useful for
/// tests and scripted sessions).
pub mod silent_waiting;

// ---------------------------------------------------------------------------
// Re-exports — bring the most-used types to the crate root for convenience
// ---------------------------------------------------------------------------

pub use types::EvalResult;
pub use types::Mode;
pub use language::Language;
pub use prompt::Prompt;
pub use waiting::Waiting;
pub use echo_language::EchoLanguage;
pub use default_prompt::DefaultPrompt;
pub use silent_waiting::SilentWaiting;
