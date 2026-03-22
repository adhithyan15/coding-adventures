//! # false — Do Nothing, Unsuccessfully
//!
//! This module implements the business logic for the `false` command.
//! The `false` utility does absolutely nothing and exits with a status
//! code of 1 (failure).
//!
//! ## Why Does This Exist?
//!
//! The `false` command is the counterpart of `true`. It exists for use
//! in shell scripts where a command that always fails is needed:
//!
//! ```text
//!     # Exit a loop
//!     while false; do
//!         echo "never runs"
//!     done
//!
//!     # Test error-handling code
//!     if false; then
//!         echo "never printed"
//!     else
//!         echo "error path works"
//!     fi
//!
//!     # Force a pipeline to fail
//!     false && echo "never printed"
//! ```
//!
//! ## Implementation
//!
//! Like `true`, there is no real business logic. The module provides
//! a single function returning the exit code for testability.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the exit code for the `false` command.
///
/// This always returns 1 (failure). The function exists so that the
/// behavior is testable — rather than calling `process::exit(1)` directly,
/// the caller can check the return value.
///
/// # Example
///
/// ```text
///     assert_eq!(exit_code(), 1);
/// ```
pub fn exit_code() -> i32 {
    1
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn false_returns_one() {
        assert_eq!(exit_code(), 1, "false should exit with code 1");
    }

    #[test]
    fn false_returns_failure() {
        // In Unix convention, non-zero means failure.
        let code = exit_code();
        assert!(code != 0, "exit code should indicate failure (non-zero), got: {}", code);
    }
}
