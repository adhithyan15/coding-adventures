//! `DefaultPrompt` — the conventional `> ` / `... ` prompt pair.
//!
//! # Conventional prompts
//!
//! Most command-line REPLs use:
//!
//! - `> ` — the **primary** prompt, indicating "I'm ready for a new statement."
//! - `... ` — the **continuation** prompt, indicating "the expression is not
//!   yet complete; give me more."
//!
//! `DefaultPrompt` implements exactly these two strings. Use it when you want
//! a working REPL quickly and don't need branded prompts.
//!
//! # Customising
//!
//! If you need different prompts, implement [`Prompt`](crate::prompt::Prompt)
//! directly:
//!
//! ```rust
//! use repl::prompt::Prompt;
//!
//! struct MyPrompt { name: String }
//! impl Prompt for MyPrompt {
//!     fn global_prompt(&self) -> String { format!("{}> ", self.name) }
//!     fn line_prompt(&self)   -> String { "... ".to_string() }
//! }
//! ```

use crate::prompt::Prompt;

/// The default prompt implementation using `"> "` and `"... "`.
///
/// This is a zero-size unit struct (no state) because the prompts are
/// compile-time constants. It is `Copy` and `Clone`.
///
/// # Examples
///
/// ```
/// use repl::default_prompt::DefaultPrompt;
/// use repl::prompt::Prompt;
///
/// let p = DefaultPrompt;
/// assert_eq!(p.global_prompt(), "> ");
/// assert_eq!(p.line_prompt(),   "... ");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultPrompt;

impl Prompt for DefaultPrompt {
    /// Returns `"> "` — the standard single-chevron primary prompt.
    ///
    /// The trailing space separates the chevron from the user's cursor,
    /// making the prompt easier to read.
    fn global_prompt(&self) -> String {
        "> ".to_string()
    }

    /// Returns `"... "` — four characters wide to align with `"> "`.
    ///
    /// Using the same visual width as the primary prompt means that
    /// multi-line input looks neatly indented:
    ///
    /// ```text
    /// > def foo(
    /// ...   x,
    /// ...   y
    /// ... ):
    /// ...   return x + y
    /// ```
    fn line_prompt(&self) -> String {
        "... ".to_string()
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::Prompt;

    #[test]
    fn test_global_prompt() {
        assert_eq!(DefaultPrompt.global_prompt(), "> ");
    }

    #[test]
    fn test_line_prompt() {
        assert_eq!(DefaultPrompt.line_prompt(), "... ");
    }

    #[test]
    fn test_prompt_is_copy() {
        let p = DefaultPrompt;
        let _q = p; // move
        let _r = p; // copy — still usable after move because it's Copy
    }
}
