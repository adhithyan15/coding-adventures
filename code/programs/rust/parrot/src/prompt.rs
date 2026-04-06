//! `ParrotPrompt` — the personality layer for the Parrot REPL.
//!
//! # What is a prompt?
//!
//! The [`repl::Prompt`] trait supplies two strings that the REPL framework
//! writes to the user:
//!
//! | Method | Shown when |
//! |--------|-----------|
//! | [`global_prompt`] | Before each new top-level expression |
//! | [`line_prompt`]   | For continuation lines in multi-line input |
//!
//! [`EchoLanguage`](repl::EchoLanguage) never uses multi-line input, so
//! `line_prompt` is provided for completeness only.
//!
//! # Design
//!
//! The parrot emoji (🦜) is the entire personality. A real REPL back-end might
//! show the current working directory, a version number, or the name of the
//! active database — all of that lives in the `Prompt` implementation, keeping
//! the runner logic generic.

use repl::Prompt;

/// The prompt implementation for the Parrot REPL.
///
/// Implements [`repl::Prompt`] with parrot-themed text and the 🦜 emoji.
///
/// # Examples
///
/// ```
/// use repl::Prompt;
/// use parrot::prompt::ParrotPrompt;
///
/// let p = ParrotPrompt;
/// assert!(p.global_prompt().contains("🦜"));
/// assert!(p.line_prompt().contains("🦜"));
/// ```
pub struct ParrotPrompt;

impl Prompt for ParrotPrompt {
    /// Returns the primary input prompt shown before each new line.
    ///
    /// The parrot emoji signals that this is the Parrot REPL; the ` > `
    /// suffix is the conventional shell-style input indicator.
    ///
    /// # Note on the trailing space
    ///
    /// The trailing space after `>` separates the prompt character from the
    /// cursor, making the line easier to read at a glance. This is the same
    /// convention used by Python (`>>> `), Ruby (`irb> `), and most Unix shells.
    fn global_prompt(&self) -> String {
        "🦜 > ".to_string()
    }

    /// Returns the continuation prompt for multi-line expressions.
    ///
    /// `EchoLanguage` never enters multi-line mode, but a well-formed
    /// `Prompt` implementation must return a sensible string here. We use
    /// the parrot emoji with `...` to keep the visual language consistent.
    fn line_prompt(&self) -> String {
        "🦜 ... ".to_string()
    }
}
