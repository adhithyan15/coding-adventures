//! # true — Do Nothing, Successfully
//!
//! This module implements the business logic for the `true` command.
//! The `true` utility does absolutely nothing and exits with a status
//! code of 0 (success).
//!
//! ## Why Does This Exist?
//!
//! The `true` command exists for use in shell scripts where a command
//! that always succeeds is needed. Common uses:
//!
//! ```text
//!     # Infinite loop
//!     while true; do
//!         echo "forever"
//!     done
//!
//!     # Placeholder for a command not yet implemented
//!     if some_condition; then
//!         true  # TODO: implement this branch
//!     fi
//!
//!     # Ensure a pipeline succeeds
//!     some_command || true
//! ```
//!
//! ## Implementation
//!
//! There is no business logic. The entire purpose of `true` is to
//! exit with code 0. The module provides a single function that
//! returns the exit code, making it testable without actually
//! calling `std::process::exit`.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the exit code for the `true` command.
///
/// This always returns 0 (success). The function exists so that the
/// behavior is testable — rather than calling `process::exit(0)` directly,
/// the caller can check the return value.
///
/// # Example
///
/// ```text
///     assert_eq!(exit_code(), 0);
/// ```
pub fn exit_code() -> i32 {
    0
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_returns_zero() {
        assert_eq!(exit_code(), 0, "true should exit with code 0");
    }

    #[test]
    fn true_returns_success() {
        // In Unix convention, 0 means success.
        // Any other value would be a failure.
        let code = exit_code();
        assert!(code == 0, "exit code should indicate success (0), got: {}", code);
    }
}
