//! # pwd — Library Module
//!
//! This module exposes the business logic functions (`get_physical_pwd` and
//! `get_logical_pwd`) so they can be tested from integration tests in the
//! `tests/` directory.
//!
//! The binary entry point is in `main.rs`, which calls these functions
//! after CLI Builder has parsed the arguments.

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
