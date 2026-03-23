//! # basename — Strip Directory and Suffix from Filenames
//!
//! This module implements the business logic for the `basename` command.
//! The `basename` utility strips the directory portion (and optionally
//! a suffix) from a pathname, leaving just the final component.
//!
//! ## How It Works
//!
//! The algorithm follows POSIX rules:
//!
//! ```text
//!     Input: "/usr/local/bin/program.sh"
//!
//!     Step 1: Remove trailing slashes
//!             "/usr/local/bin/program.sh" → same (no trailing /)
//!
//!     Step 2: Remove everything up to and including the last /
//!             "program.sh"
//!
//!     Step 3: If a suffix is provided, remove it from the end
//!             suffix = ".sh" → "program"
//! ```
//!
//! ## Edge Cases
//!
//! ```text
//!     Input           Result      Explanation
//!     ──────────────  ──────────  ────────────────────────
//!     "/"             "/"         Root is its own basename
//!     "//"            "/"         Multiple slashes → root
//!     "hello"         "hello"     No directory to strip
//!     "hello/"        "hello"     Trailing slash removed
//!     ""              ""          Empty string → empty
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Strip the directory portion from a path and optionally remove a suffix.
///
/// This implements the POSIX `basename` algorithm:
///
/// 1. If the path is empty, return empty string
/// 2. Remove all trailing slashes (unless the path is entirely slashes)
/// 3. Remove everything before and including the last slash
/// 4. If a suffix is given and the name ends with it (and the name
///    is not equal to the suffix), remove it
///
/// # Examples
///
/// ```text
///     strip_basename("/usr/bin/sort", None)         → "sort"
///     strip_basename("/usr/bin/sort", Some(".rs"))   → "sort"
///     strip_basename("program.sh", Some(".sh"))      → "program"
///     strip_basename("/", None)                      → "/"
/// ```
pub fn strip_basename(path: &str, suffix: Option<&str>) -> String {
    // --- Step 1: Handle empty path ---
    if path.is_empty() {
        return String::new();
    }

    // --- Step 2: Handle all-slashes path ---
    // If the path is nothing but slashes, the basename is "/".
    if path.chars().all(|c| c == '/') {
        return "/".to_string();
    }

    // --- Step 3: Remove trailing slashes ---
    let trimmed = path.trim_end_matches('/');

    // --- Step 4: Find the last slash and take everything after it ---
    let base = match trimmed.rfind('/') {
        Some(pos) => &trimmed[pos + 1..],
        None => trimmed,
    };

    // --- Step 5: Remove suffix if provided ---
    // The suffix is only removed if:
    //   - The base actually ends with the suffix
    //   - The base is not equal to the suffix (POSIX rule)
    let result = match suffix {
        Some(suf) if !suf.is_empty() && base.ends_with(suf) && base != suf => {
            &base[..base.len() - suf.len()]
        }
        _ => base,
    };

    result.to_string()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_path() {
        assert_eq!(strip_basename("/usr/bin/sort", None), "sort");
    }

    #[test]
    fn with_suffix() {
        assert_eq!(strip_basename("program.sh", Some(".sh")), "program");
    }

    #[test]
    fn suffix_not_matching() {
        assert_eq!(strip_basename("program.sh", Some(".txt")), "program.sh");
    }

    #[test]
    fn root_path() {
        assert_eq!(strip_basename("/", None), "/");
    }

    #[test]
    fn multiple_slashes() {
        assert_eq!(strip_basename("///", None), "/");
    }

    #[test]
    fn trailing_slash() {
        assert_eq!(strip_basename("/usr/bin/", None), "bin");
    }

    #[test]
    fn no_directory() {
        assert_eq!(strip_basename("hello", None), "hello");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_basename("", None), "");
    }

    #[test]
    fn suffix_equals_basename() {
        // POSIX: if name equals suffix, don't strip
        assert_eq!(strip_basename(".sh", Some(".sh")), ".sh");
    }

    #[test]
    fn deep_path_with_suffix() {
        assert_eq!(
            strip_basename("/home/user/docs/report.pdf", Some(".pdf")),
            "report"
        );
    }
}
