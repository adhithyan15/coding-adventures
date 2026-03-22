//! # printenv — Print Environment Variables
//!
//! This module implements the business logic for the `printenv` command.
//! The `printenv` utility prints the values of the specified environment
//! variables. If no variables are specified, it prints all of them.
//!
//! ## Behavior
//!
//! ```text
//!     Command              Output
//!     ───────────────────  ──────────────────────────────
//!     printenv             All variables, one per line
//!     printenv HOME        Value of HOME only
//!     printenv HOME PATH   Values of HOME and PATH
//!     printenv -0          All variables, NUL-separated
//! ```
//!
//! ## Exit Status
//!
//! GNU `printenv` exits with status 0 if all specified variables are
//! found, and status 1 if any are missing. Our implementation returns
//! the formatted output and lets the caller decide the exit code.
//!
//! ## NUL Termination
//!
//! The `-0` flag replaces newlines with NUL bytes as terminators.
//! This is useful for piping to `xargs -0` when variable values
//! might contain newlines.

use std::env;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get the values of the specified environment variables.
///
/// If `names` is empty, return all environment variables in
/// `KEY=VALUE` format (one per line). If specific names are given,
/// return only their values (one per line).
///
/// # Parameters
///
/// - `names`: Variable names to look up. Empty = all variables.
/// - `null_terminated`: If true, use NUL (`\0`) instead of newline
///   as the line terminator.
///
/// # Format
///
/// ```text
///     No names specified:     "HOME=/home/user\nPATH=/usr/bin\n"
///     Names specified:        "/home/user\n/usr/bin\n"
///     With -0:                "/home/user\0/usr/bin\0"
/// ```
///
/// Variables that don't exist in the environment are silently skipped.
pub fn get_env_vars(names: &[String], null_terminated: bool) -> String {
    let terminator = if null_terminated { "\0" } else { "\n" };

    if names.is_empty() {
        // --- Print all environment variables ---
        // Collect all variables, sort them for deterministic output
        // (GNU printenv doesn't sort, but sorting makes testing easier
        //  and doesn't violate the spec).
        let mut vars: Vec<(String, String)> = env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));

        let mut output = String::new();
        for (key, value) in vars {
            output.push_str(&key);
            output.push('=');
            output.push_str(&value);
            output.push_str(terminator);
        }
        output
    } else {
        // --- Print specific variables ---
        let mut output = String::new();
        for name in names {
            if let Ok(value) = env::var(name) {
                output.push_str(&value);
                output.push_str(terminator);
            }
        }
        output
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_variable() {
        // Set a test variable
        env::set_var("TEST_PRINTENV_VAR", "hello_world");
        let result = get_env_vars(&["TEST_PRINTENV_VAR".into()], false);
        assert_eq!(result, "hello_world\n");
        env::remove_var("TEST_PRINTENV_VAR");
    }

    #[test]
    fn missing_variable() {
        let result = get_env_vars(&["SURELY_THIS_DOES_NOT_EXIST_12345".into()], false);
        assert_eq!(result, "");
    }

    #[test]
    fn null_terminated() {
        env::set_var("TEST_PRINTENV_NULL", "value");
        let result = get_env_vars(&["TEST_PRINTENV_NULL".into()], true);
        assert_eq!(result, "value\0");
        env::remove_var("TEST_PRINTENV_NULL");
    }

    #[test]
    fn multiple_variables() {
        env::set_var("TEST_PE_A", "alpha");
        env::set_var("TEST_PE_B", "beta");
        let result = get_env_vars(&["TEST_PE_A".into(), "TEST_PE_B".into()], false);
        assert_eq!(result, "alpha\nbeta\n");
        env::remove_var("TEST_PE_A");
        env::remove_var("TEST_PE_B");
    }

    #[test]
    fn all_variables_includes_known() {
        env::set_var("TEST_PRINTENV_ALL", "present");
        let result = get_env_vars(&[], false);
        assert!(result.contains("TEST_PRINTENV_ALL=present"));
        env::remove_var("TEST_PRINTENV_ALL");
    }

    #[test]
    fn empty_names_returns_all() {
        let result = get_env_vars(&[], false);
        // Should contain at least some system variables
        assert!(!result.is_empty());
    }

    #[test]
    fn mixed_existing_and_missing() {
        env::set_var("TEST_PE_EXISTS", "found");
        let result = get_env_vars(
            &["TEST_PE_EXISTS".into(), "TEST_PE_MISSING_XYZ".into()],
            false,
        );
        assert_eq!(result, "found\n");
        env::remove_var("TEST_PE_EXISTS");
    }
}
