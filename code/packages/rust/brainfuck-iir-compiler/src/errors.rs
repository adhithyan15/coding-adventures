//! Errors raised by [`crate::vm::BrainfuckVM`].
//!
//! # Why a dedicated error type?
//!
//! Several distinct failure modes can occur while a Brainfuck program runs
//! under [`BrainfuckVM`](crate::vm::BrainfuckVM):
//!
//! - The data pointer walks past the end of the configured tape.
//! - The data pointer walks below cell 0.
//! - The fuel cap (`max_steps`) is exhausted by a runaway loop.
//! - The user requested JIT mode (`jit = true`) but that path is not yet
//!   wired in BF04 (it arrives in BF05).
//!
//! A single [`BrainfuckError`] type with a stable identity lets tests, REPL
//! frontends, and notebook kernels handle BF-level failures without inspecting
//! message text.
//!
//! # Example
//!
//! ```
//! use brainfuck_iir_compiler::BrainfuckError;
//!
//! let e = BrainfuckError::new("data pointer out of bounds");
//! assert!(e.to_string().contains("out of bounds"));
//! ```

use std::fmt;

// ---------------------------------------------------------------------------
// BrainfuckError
// ---------------------------------------------------------------------------

/// A Brainfuck-level execution error.
///
/// Produced by [`BrainfuckVM`](crate::vm::BrainfuckVM) when the Brainfuck
/// program misbehaves at the language level, as opposed to a Rust-level bug
/// or an internal [`vm_core::errors::VMError`].
///
/// The error message is always human-readable English describing what went
/// wrong (pointer out of bounds, step cap exceeded, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrainfuckError {
    pub message: String,
}

impl BrainfuckError {
    /// Create a new [`BrainfuckError`] with the given message.
    ///
    /// ```
    /// use brainfuck_iir_compiler::BrainfuckError;
    /// let e = BrainfuckError::new("pointer underflow");
    /// assert_eq!(e.message, "pointer underflow");
    /// ```
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BrainfuckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BrainfuckError: {}", self.message)
    }
}

impl std::error::Error for BrainfuckError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_message() {
        let e = BrainfuckError::new("something went wrong");
        assert_eq!(e.message, "something went wrong");
    }

    #[test]
    fn display_contains_message() {
        let e = BrainfuckError::new("oops");
        assert!(e.to_string().contains("oops"));
    }

    #[test]
    fn display_includes_kind_prefix() {
        let e = BrainfuckError::new("tape overflow");
        assert!(e.to_string().starts_with("BrainfuckError:"));
    }

    #[test]
    fn clone_and_eq() {
        let a = BrainfuckError::new("a");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn ne_different_messages() {
        let a = BrainfuckError::new("a");
        let b = BrainfuckError::new("b");
        assert_ne!(a, b);
    }

    #[test]
    fn debug_format_contains_message() {
        let e = BrainfuckError::new("debug test");
        assert!(format!("{e:?}").contains("debug test"));
    }

    #[test]
    fn is_std_error() {
        let e = BrainfuckError::new("test");
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn from_string_owned() {
        let msg = String::from("owned string error");
        let e = BrainfuckError::new(msg.clone());
        assert_eq!(e.message, msg);
    }
}
