//! # head — Output the First Part of Files
//!
//! This module implements the business logic for the `head` command.
//! The `head` utility outputs the first part of files. By default, it
//! prints the first 10 lines of each file to standard output.
//!
//! ## Two Modes of Operation
//!
//! `head` can operate in two mutually exclusive modes:
//!
//! ```text
//!     Mode        Flag     Description
//!     ──────────  ─────    ─────────────────────────────────
//!     Line mode   -n NUM   Print the first NUM lines (default)
//!     Byte mode   -c NUM   Print the first NUM bytes
//! ```
//!
//! These are mutually exclusive — you can use one or the other, but
//! not both at the same time.
//!
//! ## Multiple Files
//!
//! When processing multiple files, `head` prints a header before each
//! file's content:
//!
//! ```text
//!     ==> filename <==
//! ```
//!
//! The `-q` flag suppresses headers; `-v` always shows them (even for
//! a single file).

// ---------------------------------------------------------------------------
// Public API — Line Mode
// ---------------------------------------------------------------------------

/// Extract the first `n` lines from a string.
///
/// Lines are delimited by newline characters (`\n`). If the content
/// has fewer than `n` lines, the entire content is returned.
///
/// # How It Works
///
/// We iterate through the string character by character, counting
/// newlines. Once we've seen `n` newlines, we stop and return
/// everything up to (and including) that final newline.
///
/// ```text
///     Input: "alpha\nbeta\ngamma\ndelta\n"
///     n = 2
///     Output: "alpha\nbeta\n"
///
///     We count newlines:
///       'a','l','p','h','a','\n'  → count = 1
///       'b','e','t','a','\n'     → count = 2  → stop here
/// ```
///
/// # Edge Cases
///
/// - `n = 0` → returns an empty string
/// - Content without a trailing newline → the last "line" is still
///   included if we haven't reached `n` yet
pub fn head_lines(content: &str, n: usize) -> String {
    // --- Special case: zero lines requested ---
    if n == 0 {
        return String::new();
    }

    let mut count = 0;

    for (i, ch) in content.char_indices() {
        if ch == '\n' {
            count += 1;
            if count == n {
                // We've found the nth newline. Include it in the output.
                return content[..i + 1].to_string();
            }
        }
    }

    // --- Fewer than n lines in the content ---
    // Return everything we have.
    content.to_string()
}

// ---------------------------------------------------------------------------
// Public API — Byte Mode
// ---------------------------------------------------------------------------

/// Extract the first `n` bytes from a byte slice.
///
/// This is simpler than line mode — we just take a prefix of the
/// byte array. No character boundary concerns because we're operating
/// on raw bytes, just like GNU `head -c`.
///
/// # Example
///
/// ```text
///     Input: b"hello world" (11 bytes)
///     n = 5
///     Output: b"hello" (5 bytes)
/// ```
pub fn head_bytes(content: &[u8], n: usize) -> Vec<u8> {
    // --- Clamp to available length ---
    // If n exceeds the content length, we return everything.
    let take = n.min(content.len());
    content[..take].to_vec()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_two_lines() {
        let input = "alpha\nbeta\ngamma\ndelta\n";
        assert_eq!(head_lines(input, 2), "alpha\nbeta\n");
    }

    #[test]
    fn more_lines_than_available() {
        let input = "one\ntwo\n";
        assert_eq!(head_lines(input, 10), "one\ntwo\n");
    }

    #[test]
    fn zero_lines() {
        let input = "hello\nworld\n";
        assert_eq!(head_lines(input, 0), "");
    }

    #[test]
    fn no_trailing_newline() {
        let input = "alpha\nbeta\ngamma";
        assert_eq!(head_lines(input, 5), "alpha\nbeta\ngamma");
    }

    #[test]
    fn single_line() {
        assert_eq!(head_lines("hello\n", 1), "hello\n");
    }

    #[test]
    fn empty_content() {
        assert_eq!(head_lines("", 5), "");
    }

    #[test]
    fn first_five_bytes() {
        let input = b"hello world";
        assert_eq!(head_bytes(input, 5), b"hello");
    }

    #[test]
    fn more_bytes_than_available() {
        let input = b"hi";
        assert_eq!(head_bytes(input, 100), b"hi");
    }

    #[test]
    fn zero_bytes() {
        let input = b"hello";
        assert_eq!(head_bytes(input, 0), b"");
    }

    #[test]
    fn empty_bytes() {
        let input: &[u8] = b"";
        assert_eq!(head_bytes(input, 5), b"");
    }
}
