//! # unix-tools — Library Module
//!
//! This crate provides the business logic for a collection of Unix
//! command-line tools, all powered by [CLI Builder](../../../packages/rust/cli-builder/).
//!
//! Each tool lives in its own module:
//!
//! - **pwd** — Print the current working directory (logical or physical)
//! - **echo** — Display a line of text, with optional escape interpretation
//! - **cat** — Concatenate and display files, with numbering and formatting
//! - **wc** — Count lines, words, bytes, and characters
//! - **true/false** — Exit with success (0) or failure (1) status
//!
//! The `true` and `false` tools have no business logic — they simply exit.
//! They're included for completeness and to verify CLI Builder handles
//! minimal specs correctly.
//!
//! ## Architecture
//!
//! ```text
//!     JSON spec (e.g., pwd.json)
//!         │
//!         ▼
//!     CLI Builder (parsing, validation, help)
//!         │
//!         ▼
//!     This crate (business logic)
//!         │
//!         ▼
//!     main.rs (I/O and exit codes)
//! ```

// ---------------------------------------------------------------------------
// Tool modules
// ---------------------------------------------------------------------------

pub mod echo_tool;
pub mod cat_tool;
pub mod wc_tool;

// ---------------------------------------------------------------------------
// pwd — Print Working Directory
// ---------------------------------------------------------------------------
// The pwd functions live directly in lib.rs for historical reasons (they
// were the first tool implemented). New tools get their own modules.

use std::path::PathBuf;

/// Return the physical working directory with all symlinks resolved.
///
/// This calls `std::env::current_dir()` followed by `.canonicalize()`,
/// which follows every symlink in the path to produce the canonical
/// filesystem path.
///
/// # Example
///
/// If `/home` is a symlink to `/usr/home` and the cwd is `/home/user`:
///
/// ```text
/// get_physical_pwd() => "/usr/home/user"
/// ```
pub fn get_physical_pwd() -> Result<String, String> {
    let cwd = std::env::current_dir()
        .map_err(|e| format!("pwd: cannot determine current directory: {}", e))?;
    let canonical = cwd
        .canonicalize()
        .map_err(|e| format!("pwd: cannot canonicalize current directory: {}", e))?;
    Ok(canonical.to_string_lossy().into_owned())
}

/// Return the logical working directory.
///
/// The logical path comes from the `$PWD` environment variable, which
/// the shell maintains as the user navigates — including through symlinks.
///
/// If `$PWD` is not set or is stale (doesn't match the real cwd), we
/// fall back to the physical path. This matches POSIX behavior: the
/// logical path is best-effort, never wrong.
///
/// ## Validation
///
/// We don't blindly trust `$PWD`. We verify that resolving `$PWD`
/// through symlinks yields the same directory as resolving `.` through
/// symlinks. If they differ, `$PWD` is stale and we ignore it.
///
/// ```text
/// $PWD = "/home/user"
///   └── realpath("/home/user") == "/usr/home/user"
///   └── realpath(".")          == "/usr/home/user"
///   └── They match ✓ → return "/home/user"
/// ```
pub fn get_logical_pwd() -> Result<String, String> {
    // --- Step 1: Read $PWD ---
    if let Ok(env_pwd) = std::env::var("PWD") {
        // --- Step 2: Validate $PWD ---
        let env_path = PathBuf::from(&env_pwd);
        if let Ok(env_real) = env_path.canonicalize() {
            if let Ok(cwd) = std::env::current_dir() {
                if let Ok(cwd_real) = cwd.canonicalize() {
                    if env_real == cwd_real {
                        return Ok(env_pwd);
                    }
                }
            }
        }
    }

    // --- Fallback: physical path ---
    get_physical_pwd()
}
