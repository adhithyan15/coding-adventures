//! # tail — Output the Last Part of Files
//!
//! This module implements the business logic for the `tail` command.
//! The `tail` utility outputs the last part of files. By default, it
//! prints the last 10 lines of each file to standard output.
//!
//! ## Two Addressing Modes
//!
//! GNU `tail` supports two ways to specify which lines to output:
//!
//! ```text
//!     Syntax     Meaning
//!     ────────   ─────────────────────────────────────────
//!     -n NUM     Output the last NUM lines (count from end)
//!     -n +NUM    Output starting with line NUM (count from start)
//! ```
//!
//! The `+` prefix flips the behavior: instead of counting backward
//! from the end, it counts forward from the beginning.
//!
//! ## Example
//!
//! Given a file with lines A, B, C, D, E:
//!
//! ```text
//!     tail -n 2    → D, E        (last 2 lines)
//!     tail -n +3   → C, D, E     (from line 3 onward)
//! ```

// ---------------------------------------------------------------------------
// Public API — Count from End
// ---------------------------------------------------------------------------

/// Extract the last `n` lines from a string.
///
/// Lines are delimited by newline characters. If the content has
/// fewer than `n` lines, the entire content is returned.
///
/// # Algorithm
///
/// We split the content into lines, then take the last `n`. This is
/// straightforward and readable, trading a small amount of memory
/// for clarity.
///
/// ```text
///     Input: "alpha\nbeta\ngamma\ndelta\n"
///     n = 2
///
///     Lines: ["alpha", "beta", "gamma", "delta", ""]
///     (trailing empty element from final \n)
///
///     Last 2 real lines: ["gamma", "delta"]
///     Output: "gamma\ndelta\n"
/// ```
pub fn tail_lines(content: &str, n: usize) -> String {
    // --- Special case: zero lines requested ---
    if n == 0 {
        return String::new();
    }

    // --- Split into lines ---
    // We need to handle the trailing newline carefully. If the content
    // ends with \n, split() produces an empty trailing element that
    // we should not count as a real line.
    let has_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<&str> = content.split('\n').collect();

    // Remove the phantom empty element from trailing newline
    if has_trailing_newline && lines.last() == Some(&"") {
        lines.pop();
    }

    // --- Take the last n lines ---
    let start = if lines.len() > n { lines.len() - n } else { 0 };
    let selected = &lines[start..];

    // --- Reconstruct with newlines ---
    let mut result = selected.join("\n");
    if has_trailing_newline || !content.is_empty() {
        result.push('\n');
    }

    result
}

// ---------------------------------------------------------------------------
// Public API — Count from Start
// ---------------------------------------------------------------------------

/// Extract lines starting from line number `n` (1-indexed).
///
/// This implements the `+NUM` syntax of GNU tail. Line numbering
/// starts at 1, matching the convention used by `tail -n +N`.
///
/// ```text
///     Input: "alpha\nbeta\ngamma\ndelta\n"
///     n = 3  (start from line 3)
///
///     Lines: ["alpha", "beta", "gamma", "delta"]
///     Skip first 2 (n-1): ["gamma", "delta"]
///     Output: "gamma\ndelta\n"
/// ```
///
/// # Edge Cases
///
/// - `n = 1` → return entire content (start from line 1 = everything)
/// - `n = 0` → treated as `n = 1` (same as GNU behavior)
/// - `n` > number of lines → return empty string
pub fn tail_from_line(content: &str, n: usize) -> String {
    // --- Normalize: n=0 behaves like n=1 in GNU tail ---
    let start_line = if n == 0 { 1 } else { n };

    let has_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<&str> = content.split('\n').collect();

    // Remove phantom trailing element
    if has_trailing_newline && lines.last() == Some(&"") {
        lines.pop();
    }

    // --- Skip the first (n-1) lines ---
    let skip = start_line - 1;
    if skip >= lines.len() {
        return String::new();
    }

    let selected = &lines[skip..];
    let mut result = selected.join("\n");
    if has_trailing_newline {
        result.push('\n');
    }

    result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_two_lines() {
        let input = "alpha\nbeta\ngamma\ndelta\n";
        assert_eq!(tail_lines(input, 2), "gamma\ndelta\n");
    }

    #[test]
    fn more_lines_than_available() {
        let input = "one\ntwo\n";
        assert_eq!(tail_lines(input, 10), "one\ntwo\n");
    }

    #[test]
    fn zero_lines() {
        assert_eq!(tail_lines("hello\nworld\n", 0), "");
    }

    #[test]
    fn single_line() {
        assert_eq!(tail_lines("hello\n", 1), "hello\n");
    }

    #[test]
    fn empty_content() {
        assert_eq!(tail_lines("", 5), "");
    }

    #[test]
    fn from_line_three() {
        let input = "alpha\nbeta\ngamma\ndelta\nepsilon\n";
        assert_eq!(tail_from_line(input, 3), "gamma\ndelta\nepsilon\n");
    }

    #[test]
    fn from_line_one_is_everything() {
        let input = "alpha\nbeta\n";
        assert_eq!(tail_from_line(input, 1), "alpha\nbeta\n");
    }

    #[test]
    fn from_line_beyond_end() {
        let input = "alpha\nbeta\n";
        assert_eq!(tail_from_line(input, 10), "");
    }

    #[test]
    fn from_line_zero_is_everything() {
        let input = "alpha\nbeta\n";
        assert_eq!(tail_from_line(input, 0), "alpha\nbeta\n");
    }

    #[test]
    fn last_line_no_trailing_newline() {
        let input = "alpha\nbeta\ngamma";
        assert_eq!(tail_lines(input, 1), "gamma\n");
    }
}
