//! # dirname — Strip Last Component from File Name
//!
//! This module implements the business logic for the `dirname` command.
//! The `dirname` utility strips the last component from a file name,
//! outputting the directory portion of a path.
//!
//! ## How It Works
//!
//! The algorithm follows POSIX rules, which are the inverse of `basename`:
//!
//! ```text
//!     Input: "/usr/local/bin/program"
//!
//!     Step 1: Remove trailing slashes
//!             "/usr/local/bin/program" → same
//!
//!     Step 2: Remove everything after the last /
//!             "/usr/local/bin"
//!
//!     Step 3: Remove trailing slashes from result
//!             "/usr/local/bin" → same
//! ```
//!
//! ## Truth Table
//!
//! ```text
//!     Input              Output     Explanation
//!     ─────────────────  ─────────  ──────────────────────
//!     "/usr/bin/sort"    "/usr/bin" Remove last component
//!     "/usr/bin/"        "/usr"     Trailing / stripped first
//!     "hello"            "."        No directory → current dir
//!     "/"                "/"        Root is its own dirname
//!     ""                 "."        Empty → current dir
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Strip the last component from a path, returning the directory.
///
/// This implements the POSIX `dirname` algorithm:
///
/// 1. If the path is empty, return "."
/// 2. Remove trailing slashes
/// 3. If no slash remains, return "." (it was just a filename)
/// 4. Remove the last component (everything after the last /)
/// 5. Remove trailing slashes from the result
/// 6. If the result is empty, return "/"
///
/// # Examples
///
/// ```text
///     strip_dirname("/usr/bin/sort") → "/usr/bin"
///     strip_dirname("hello")         → "."
///     strip_dirname("/")             → "/"
///     strip_dirname("a/b")           → "a"
/// ```
pub fn strip_dirname(path: &str) -> String {
    // --- Step 1: Handle empty path ---
    if path.is_empty() {
        return ".".to_string();
    }

    // --- Step 2: Handle all-slashes path ---
    if path.chars().all(|c| c == '/') {
        return "/".to_string();
    }

    // --- Step 3: Remove trailing slashes ---
    let trimmed = path.trim_end_matches('/');

    // --- Step 4: Find the last slash ---
    match trimmed.rfind('/') {
        None => {
            // No slash at all — the path is just a filename.
            // dirname of a bare filename is "." (current directory).
            ".".to_string()
        }
        Some(pos) => {
            // --- Step 5: Take everything before the last component ---
            let dir = &trimmed[..pos];

            // --- Step 6: Remove trailing slashes from result ---
            let dir_trimmed = dir.trim_end_matches('/');

            // --- Step 7: Handle empty result ---
            // If trimming slashes leaves nothing, the directory is "/"
            if dir_trimmed.is_empty() {
                "/".to_string()
            } else {
                dir_trimmed.to_string()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_path() {
        assert_eq!(strip_dirname("/usr/bin/sort"), "/usr/bin");
    }

    #[test]
    fn trailing_slash() {
        assert_eq!(strip_dirname("/usr/bin/"), "/usr");
    }

    #[test]
    fn bare_filename() {
        assert_eq!(strip_dirname("hello"), ".");
    }

    #[test]
    fn root_path() {
        assert_eq!(strip_dirname("/"), "/");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_dirname(""), ".");
    }

    #[test]
    fn relative_path() {
        assert_eq!(strip_dirname("a/b"), "a");
    }

    #[test]
    fn file_in_root() {
        assert_eq!(strip_dirname("/hello"), "/");
    }

    #[test]
    fn deep_path() {
        assert_eq!(strip_dirname("/a/b/c/d"), "/a/b/c");
    }

    #[test]
    fn double_slash() {
        assert_eq!(strip_dirname("//"), "/");
    }

    #[test]
    fn multiple_trailing_slashes() {
        assert_eq!(strip_dirname("/usr/bin///"), "/usr");
    }
}
