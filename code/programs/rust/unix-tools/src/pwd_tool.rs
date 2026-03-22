//! # pwd — Print Working Directory
//!
//! This module implements the business logic for the `pwd` command.
//! The `pwd` utility prints the absolute pathname of the current
//! working directory to standard output.
//!
//! ## Logical vs Physical Paths
//!
//! There are two ways to determine the current directory:
//!
//! ```text
//!     Mode       Flag   Behavior
//!     ────────   ─────  ──────────────────────────────────────
//!     Logical    -L     Use the $PWD environment variable.
//!                       This preserves symlink names as the
//!                       user typed them.
//!     Physical   -P     Resolve all symlinks and print the
//!                       canonical filesystem path.
//! ```
//!
//! The default mode is logical (`-L`). If `$PWD` is not set or is
//! stale (doesn't match the actual directory), logical mode falls
//! back to physical mode. This matches POSIX behavior.
//!
//! ## Example
//!
//! If `/home` is a symlink to `/usr/home`:
//!
//! ```text
//!     $ cd /home/user
//!     $ pwd -L
//!     /home/user        (what $PWD says)
//!     $ pwd -P
//!     /usr/home/user    (what the filesystem says)
//! ```
//!
//! ## Design Note
//!
//! The actual implementation of `get_logical_pwd()` and
//! `get_physical_pwd()` lives in `lib.rs` at the crate root,
//! since pwd was the first tool implemented. This module re-exports
//! those functions for consistency with the tool module pattern.

// ---------------------------------------------------------------------------
// Re-exports from crate root
// ---------------------------------------------------------------------------
// The pwd functions were originally defined in lib.rs before the
// module-per-tool pattern was established. We re-export them here
// so that test code can use `unix_tools::pwd_tool::get_logical_pwd()`
// consistently with other tools.

pub use crate::{get_logical_pwd, get_physical_pwd};

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_pwd_returns_absolute_path() {
        let result = get_physical_pwd();
        assert!(result.is_ok(), "get_physical_pwd should succeed");
        let path = result.unwrap();
        assert!(
            path.starts_with('/'),
            "physical pwd should be absolute, got: {}",
            path
        );
    }

    #[test]
    fn logical_pwd_returns_absolute_path() {
        let result = get_logical_pwd();
        assert!(result.is_ok(), "get_logical_pwd should succeed");
        let path = result.unwrap();
        assert!(
            path.starts_with('/'),
            "logical pwd should be absolute, got: {}",
            path
        );
    }

    #[test]
    fn physical_pwd_is_nonempty() {
        let result = get_physical_pwd().unwrap();
        assert!(!result.is_empty(), "physical pwd should not be empty");
    }

    #[test]
    fn logical_pwd_is_nonempty() {
        let result = get_logical_pwd().unwrap();
        assert!(!result.is_empty(), "logical pwd should not be empty");
    }
}
