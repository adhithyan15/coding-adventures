//! # whoami — Print Effective User Name
//!
//! This module implements the business logic for the `whoami` command.
//! The `whoami` utility prints the user name associated with the
//! current effective user ID.
//!
//! ## How It Works
//!
//! On Unix systems, every process runs with an "effective user ID"
//! (EUID). This determines what permissions the process has. The
//! `whoami` command translates this numeric ID into a human-readable
//! user name.
//!
//! ```text
//!     $ whoami
//!     alice
//!
//!     $ sudo whoami
//!     root
//! ```
//!
//! ## Implementation Strategy
//!
//! We use the `USER` environment variable rather than calling libc
//! functions directly. This provides cross-platform compatibility
//! and is simpler to test. On most Unix systems, `$USER` reflects
//! the effective user name.
//!
//! If `$USER` is not set, we fall back to `$LOGNAME`, and finally
//! return an error if neither is available.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the effective user name.
///
/// This checks environment variables in the following order:
///
/// 1. `$USER` — set by most shells and login programs
/// 2. `$LOGNAME` — POSIX-defined login name variable
///
/// If neither is set, returns an error.
///
/// # Example
///
/// ```text
///     // With USER=alice in the environment:
///     get_username() => Ok("alice")
/// ```
pub fn get_username() -> Result<String, String> {
    // --- Strategy 1: $USER ---
    // This is the most common way to determine the current user.
    // Most shells (bash, zsh, fish) set this automatically.
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            return Ok(user);
        }
    }

    // --- Strategy 2: $LOGNAME ---
    // POSIX defines LOGNAME as the login name. It's a reasonable
    // fallback when USER is not set (e.g., in cron jobs).
    if let Ok(logname) = std::env::var("LOGNAME") {
        if !logname.is_empty() {
            return Ok(logname);
        }
    }

    // --- No username found ---
    Err("whoami: cannot determine user name".to_string())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn returns_a_username() {
        // In any reasonable test environment, $USER should be set.
        let result = get_username();
        assert!(result.is_ok(), "get_username should succeed in test environment");
        let name = result.unwrap();
        assert!(!name.is_empty(), "username should not be empty");
    }

    #[cfg(unix)]
    #[test]
    fn username_has_no_whitespace() {
        // Unix usernames should not contain spaces or newlines.
        let name = get_username().unwrap();
        assert!(
            !name.contains(' ') && !name.contains('\n'),
            "username should not contain whitespace, got: '{}'",
            name
        );
    }

    #[cfg(unix)]
    #[test]
    fn username_is_not_empty() {
        let name = get_username().unwrap();
        assert!(!name.is_empty(), "username should not be empty");
    }
}
