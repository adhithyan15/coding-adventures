//! The `Prompt` trait — customisable input prompts.
//!
//! # Two kinds of prompt
//!
//! Most REPLs show two distinct prompts:
//!
//! 1. **Global prompt** (`> ` by convention) — shown when the REPL is waiting
//!    for the *start* of a new expression. This is the familiar shell dollar
//!    sign, Python `>>>`, or Elixir `iex>`.
//!
//! 2. **Line (continuation) prompt** (`... ` by convention) — shown when the
//!    user has started a multi-line expression and needs to type more. Python
//!    uses `...`, Ruby uses `?>`, and so on.
//!
//! Separating them into a trait rather than hard-coding strings lets language
//! backends customise the look without touching the runner logic. A Lisp REPL
//! might use `λ> ` while a SQL REPL might use `SQL> `.
//!
//! # Implementing `Prompt`
//!
//! ```rust
//! use repl::prompt::Prompt;
//!
//! struct LispPrompt;
//!
//! impl Prompt for LispPrompt {
//!     fn global_prompt(&self) -> String { "λ> ".to_string() }
//!     fn line_prompt(&self)   -> String { "   ".to_string() }
//! }
//! ```

/// Provides the text strings displayed to the user before each input line.
///
/// Both methods return owned `String`s (rather than `&str`) so that
/// implementations can build prompts dynamically — for example, including a
/// line number, the current working directory, or the active namespace.
///
/// # Thread safety
///
/// Must be `Send + Sync` because the runner may call these methods from the
/// main thread while evaluation is happening on a worker thread.
pub trait Prompt: Send + Sync {
    /// The prompt shown at the start of a fresh input expression.
    ///
    /// Displayed immediately before the user can start typing a new
    /// statement or expression. Should end with a space for readability.
    ///
    /// # Example return values
    ///
    /// - `"> "`  (generic)
    /// - `">>> "` (Python style)
    /// - `"iex(1)> "` (Elixir style with line counter)
    fn global_prompt(&self) -> String;

    /// The continuation prompt shown when a multi-line input is in progress.
    ///
    /// Displayed after the user presses Enter mid-expression (e.g., inside
    /// an unclosed parenthesis or a block construct). Should be the same
    /// width as `global_prompt` so that continuation lines line up visually.
    ///
    /// # Example return values
    ///
    /// - `"... "` (Python, generic)
    /// - `"... "` (same width as `">>> "`)
    fn line_prompt(&self) -> String;
}
