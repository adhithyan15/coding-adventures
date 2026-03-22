//! # logname — Print Login Name
//!
//! This module implements the business logic for the `logname` command.
//! The `logname` utility prints the login name of the current user,
//! as recorded by the system when the user logged in.
//!
//! ## logname vs whoami
//!
//! These two commands are often confused. Here's the difference:
//!
//! ```text
//!     Command    What It Reports         Source
//!     ─────────  ─────────────────────   ──────────────────
//!     whoami     Effective user name     Current EUID
//!     logname    Login user name         Original login
//! ```
//!
//! In practice, they differ when you use `su` or `sudo`:
//!
//! ```text
//!     $ whoami
//!     alice
//!     $ sudo su bob
//!     $ whoami
//!     bob          (effective user changed)
//!     $ logname
//!     alice        (login user unchanged)
//! ```
//!
//! ## Implementation
//!
//! We use the `LOGNAME` environment variable, which POSIX requires
//! to be set to the login name. As a fallback, we check `USER`.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the login name of the current user.
///
/// This checks environment variables in the following order:
///
/// 1. `$LOGNAME` — POSIX-defined login name variable
/// 2. `$USER` — fallback for systems that don't set LOGNAME
///
/// If neither is set, returns an error.
///
/// # Example
///
/// ```text
///     // With LOGNAME=alice in the environment:
///     get_logname() => Ok("alice")
/// ```
pub fn get_logname() -> Result<String, String> {
    // --- Strategy 1: $LOGNAME ---
    // POSIX mandates that the login system sets this variable.
    // It should reflect the original login user, not the effective user.
    if let Ok(logname) = std::env::var("LOGNAME") {
        if !logname.is_empty() {
            return Ok(logname);
        }
    }

    // --- Strategy 2: $USER ---
    // Fallback for systems where LOGNAME is not set. On most modern
    // Unix systems, USER and LOGNAME have the same value unless
    // su/sudo has been used.
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            return Ok(user);
        }
    }

    // --- No login name found ---
    Err("logname: no login name".to_string())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_login_name() {
        // In any reasonable test environment, $LOGNAME or $USER should be set.
        let result = get_logname();
        assert!(result.is_ok(), "get_logname should succeed in test environment");
        let name = result.unwrap();
        assert!(!name.is_empty(), "login name should not be empty");
    }

    #[test]
    fn login_name_has_no_whitespace() {
        let name = get_logname().unwrap();
        assert!(
            !name.contains(' ') && !name.contains('\n'),
            "login name should not contain whitespace, got: '{}'",
            name
        );
    }

    #[test]
    fn login_name_is_consistent() {
        // Calling get_logname twice should return the same value.
        let first = get_logname().unwrap();
        let second = get_logname().unwrap();
        assert_eq!(first, second, "login name should be consistent across calls");
    }
}
