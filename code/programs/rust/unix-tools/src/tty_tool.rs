//! # tty — Print Terminal Name
//!
//! This module implements the business logic for the `tty` command.
//! The `tty` utility prints the file name of the terminal connected
//! to standard input.
//!
//! ## How Terminals Work
//!
//! In Unix, terminals are represented as special files (device nodes)
//! in the filesystem. When you open a terminal emulator, the system
//! creates a pseudo-terminal pair:
//!
//! ```text
//!     /dev/pts/0    ← your terminal's device file
//!     /dev/tty      ← always refers to the controlling terminal
//! ```
//!
//! The `tty` command tells you which device file is connected to
//! stdin. If stdin is not a terminal (e.g., it's a pipe or file),
//! `tty` prints "not a tty" and exits with status 1.
//!
//! ## Flags
//!
//! ```text
//!     Flag   Effect
//!     ─────  ─────────────────────────────────────
//!     -s     Silent mode: print nothing, just set
//!            the exit status (0 = tty, 1 = not)
//! ```
//!
//! ## Exit Status
//!
//! ```text
//!     0    Standard input is a terminal
//!     1    Standard input is not a terminal
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Result of checking whether stdin is a terminal.
///
/// This struct holds both the terminal name (if any) and the exit
/// status, allowing the caller to decide what to print based on
/// whether silent mode is enabled.
#[derive(Debug, Clone, PartialEq)]
pub struct TtyResult {
    /// The terminal device name (e.g., "/dev/pts/0"), or "not a tty"
    /// if stdin is not a terminal.
    pub name: String,
    /// Exit status: 0 if stdin is a terminal, 1 if not.
    pub exit_code: i32,
}

/// Check whether stdin is a terminal and return its name.
///
/// This function uses the `libc::isatty()` function to determine
/// whether file descriptor 0 (stdin) is connected to a terminal.
/// If it is, we use `libc::ttyname()` to get the device file path.
///
/// # Returns
///
/// A `TtyResult` containing the terminal name and exit status.
///
/// # Example
///
/// ```text
///     // When stdin is /dev/pts/0:
///     check_tty() => TtyResult { name: "/dev/pts/0", exit_code: 0 }
///
///     // When stdin is a pipe:
///     check_tty() => TtyResult { name: "not a tty", exit_code: 1 }
/// ```
#[cfg(unix)]
pub fn check_tty() -> TtyResult {
    // --- Step 1: Check if stdin is a terminal ---
    // libc::isatty() returns 1 if the file descriptor refers to a
    // terminal, 0 otherwise. We pass 0 for stdin.
    let is_tty = unsafe { libc::isatty(0) } == 1;

    if !is_tty {
        return TtyResult {
            name: "not a tty".to_string(),
            exit_code: 1,
        };
    }

    // --- Step 2: Get the terminal name ---
    // libc::ttyname() returns a pointer to a static string containing
    // the terminal device path. We convert it to a Rust String.
    let name = unsafe {
        let ptr = libc::ttyname(0);
        if ptr.is_null() {
            // ttyname failed despite isatty succeeding — unusual but possible
            "not a tty".to_string()
        } else {
            std::ffi::CStr::from_ptr(ptr)
                .to_string_lossy()
                .into_owned()
        }
    };

    if name.is_empty() {
        TtyResult {
            name: "not a tty".to_string(),
            exit_code: 1,
        }
    } else {
        TtyResult {
            name,
            exit_code: 0,
        }
    }
}

/// Non-Unix stub so the code compiles on all platforms.
#[cfg(not(unix))]
pub fn check_tty() -> TtyResult {
    TtyResult {
        name: "not a tty".to_string(),
        exit_code: 1,
    }
}

/// Format the tty output based on silent mode.
///
/// In normal mode, we print the terminal name (or "not a tty").
/// In silent mode, we print nothing — the exit code says it all.
///
/// # Parameters
///
/// - `result`: The result from `check_tty()`
/// - `silent`: Whether `-s` (silent) mode is enabled
///
/// # Returns
///
/// A `String` to print to stdout. May be empty in silent mode.
pub fn format_tty_output(result: &TtyResult, silent: bool) -> String {
    if silent {
        String::new()
    } else {
        format!("{}\n", result.name)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_tty_returns_result() {
        // In a test environment, stdin is usually NOT a tty (it's
        // connected to the test runner). So we expect "not a tty".
        let result = check_tty();
        // We can't guarantee whether it's a tty or not, but the
        // result should be consistent.
        assert!(
            result.exit_code == 0 || result.exit_code == 1,
            "exit code should be 0 or 1"
        );
    }

    #[test]
    fn not_a_tty_in_test() {
        // When running under `cargo test`, stdin is typically not a tty.
        let result = check_tty();
        // This is almost always true in CI/test environments:
        assert_eq!(result.name, "not a tty");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn format_normal_mode_with_tty() {
        let result = TtyResult {
            name: "/dev/pts/0".to_string(),
            exit_code: 0,
        };
        assert_eq!(format_tty_output(&result, false), "/dev/pts/0\n");
    }

    #[test]
    fn format_normal_mode_not_tty() {
        let result = TtyResult {
            name: "not a tty".to_string(),
            exit_code: 1,
        };
        assert_eq!(format_tty_output(&result, false), "not a tty\n");
    }

    #[test]
    fn format_silent_mode_empty() {
        let result = TtyResult {
            name: "/dev/pts/0".to_string(),
            exit_code: 0,
        };
        assert_eq!(format_tty_output(&result, true), "");
    }

    #[test]
    fn format_silent_mode_not_tty() {
        let result = TtyResult {
            name: "not a tty".to_string(),
            exit_code: 1,
        };
        assert_eq!(format_tty_output(&result, true), "");
    }

    #[test]
    fn tty_result_exit_code_matches_name() {
        let result = check_tty();
        if result.name == "not a tty" {
            assert_eq!(result.exit_code, 1);
        } else {
            assert_eq!(result.exit_code, 0);
        }
    }
}
