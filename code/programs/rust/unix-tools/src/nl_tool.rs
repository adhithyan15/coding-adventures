//! # nl — Number Lines of Files
//!
//! This module implements the business logic for the `nl` command.
//! nl reads input and writes it to output with line numbers prepended.
//!
//! ## Numbering Styles
//!
//! ```text
//!     Style   Meaning
//!     ──────  ───────────────────────────
//!     a       Number all lines
//!     t       Number only non-empty lines (default)
//!     n       No numbering
//! ```
//!
//! ## Number Formats
//!
//! ```text
//!     Format  Example
//!     ──────  ──────────
//!     ln      "1     "   (left-justified)
//!     rn      "     1"   (right-justified, default)
//!     rz      "000001"   (right-justified, zero-padded)
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Determine whether a line should be numbered based on the style.
///
/// - "a" = all lines
/// - "t" = non-empty lines only (default)
/// - "n" = no lines
pub fn should_number(line: &str, style: &str) -> bool {
    match style {
        "a" => true,
        "t" => !line.trim().is_empty(),
        "n" => false,
        _ => !line.trim().is_empty(), // treat unknown as "t"
    }
}

/// Format a line number according to the specified format.
///
/// - "ln" = left-justified, space-padded
/// - "rn" = right-justified, space-padded (default)
/// - "rz" = right-justified, zero-padded
pub fn format_line_number(num: usize, width: usize, format: &str) -> String {
    match format {
        "ln" => format!("{:<width$}", num, width = width),
        "rz" => format!("{:0>width$}", num, width = width),
        _ => format!("{:>width$}", num, width = width), // "rn" default
    }
}

/// Number the lines of a given content string.
///
/// # Parameters
/// - `content`: the input text
/// - `body_style`: numbering style for body lines ("a", "t", "n")
/// - `num_format`: number format ("ln", "rn", "rz")
/// - `num_width`: width of the number field
/// - `increment`: line number increment
/// - `starting_num`: first line number
/// - `separator`: string between number and line content
pub fn number_lines(
    content: &str,
    body_style: &str,
    num_format: &str,
    num_width: usize,
    increment: usize,
    starting_num: usize,
    separator: &str,
) -> String {
    let mut result = String::new();
    let mut line_num = starting_num;

    for line in content.lines() {
        if should_number(line, body_style) {
            let num_str = format_line_number(line_num, num_width, num_format);
            result.push_str(&format!("{}{}{}\n", num_str, separator, line));
            line_num += increment;
        } else {
            // Print unnumbered line with padding for alignment.
            let padding = " ".repeat(num_width + separator.len());
            result.push_str(&format!("{}{}\n", padding, line));
        }
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
    fn should_number_all() {
        assert!(should_number("hello", "a"));
        assert!(should_number("", "a"));
    }

    #[test]
    fn should_number_non_empty() {
        assert!(should_number("hello", "t"));
        assert!(!should_number("", "t"));
        assert!(!should_number("   ", "t"));
    }

    #[test]
    fn should_number_none() {
        assert!(!should_number("hello", "n"));
    }

    #[test]
    fn format_rn() {
        assert_eq!(format_line_number(1, 6, "rn"), "     1");
    }

    #[test]
    fn format_ln() {
        assert_eq!(format_line_number(1, 6, "ln"), "1     ");
    }

    #[test]
    fn format_rz() {
        assert_eq!(format_line_number(1, 6, "rz"), "000001");
    }

    #[test]
    fn number_lines_default() {
        let result = number_lines("hello\nworld\n", "t", "rn", 6, 1, 1, "\t");
        assert!(result.contains("     1\thello"));
        assert!(result.contains("     2\tworld"));
    }

    #[test]
    fn number_all_lines() {
        let result = number_lines("hello\n\nworld\n", "a", "rn", 6, 1, 1, "\t");
        // All three lines should be numbered.
        assert!(result.contains("     1"));
        assert!(result.contains("     2"));
        assert!(result.contains("     3"));
    }

    #[test]
    fn skip_empty_lines() {
        let result = number_lines("hello\n\nworld\n", "t", "rn", 6, 1, 1, "\t");
        // "hello" numbered as 1, empty unnumbered, "world" as 2.
        assert!(result.contains("     1\thello"));
        assert!(result.contains("     2\tworld"));
    }

    #[test]
    fn custom_increment() {
        let result = number_lines("a\nb\nc\n", "a", "rn", 6, 2, 1, "\t");
        assert!(result.contains("     1\ta"));
        assert!(result.contains("     3\tb"));
        assert!(result.contains("     5\tc"));
    }

    #[test]
    fn custom_starting_number() {
        let result = number_lines("a\nb\n", "a", "rn", 6, 1, 10, "\t");
        assert!(result.contains("    10\ta"));
        assert!(result.contains("    11\tb"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(number_lines("", "t", "rn", 6, 1, 1, "\t"), "");
    }
}
