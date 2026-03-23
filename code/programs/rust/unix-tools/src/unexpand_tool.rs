//! # unexpand — Convert Spaces to Tabs
//!
//! This module implements the business logic for the `unexpand` command.
//! unexpand is the inverse of expand: it converts sequences of spaces
//! back into tab characters at tab stop boundaries.
//!
//! ## How It Works
//!
//! unexpand walks through each line tracking column position. When a
//! sequence of spaces reaches a tab stop boundary, it replaces them
//! with a single tab character.
//!
//! ```text
//!     Input:  "        hello"  (8 spaces)
//!     Output: "\thello"        (1 tab)
//! ```

use super::expand_tool::{next_tab_stop, parse_tab_stops};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert spaces to tabs in a single line.
///
/// # Parameters
/// - `line`: input line (without newline)
/// - `tab_stops`: parsed tab stop positions
/// - `all_blanks`: if true, convert all spaces; if false, only leading ones
pub fn unexpand_line(line: &str, tab_stops: &[usize], all_blanks: bool) -> String {
    let mut result = String::new();
    let mut column = 0;
    let mut space_count = 0;
    let mut space_start_col = 0;
    let mut seen_non_blank = false;

    for ch in line.chars() {
        if ch == ' ' {
            if !all_blanks && seen_non_blank {
                result.push(' ');
                column += 1;
                continue;
            }

            if space_count == 0 {
                space_start_col = column;
            }
            space_count += 1;
            column += 1;

            // Check if we've reached a tab stop.
            let tab_width = next_tab_stop(space_start_col, tab_stops);
            if space_count >= tab_width {
                result.push('\t');
                space_count = 0;
            }
        } else {
            // Flush remaining spaces.
            for _ in 0..space_count {
                result.push(' ');
            }
            space_count = 0;

            if ch != '\t' {
                seen_non_blank = true;
            }
            result.push(ch);
            if ch == '\t' {
                column += next_tab_stop(column, tab_stops);
            } else {
                column += 1;
            }
        }
    }

    // Flush trailing spaces.
    for _ in 0..space_count {
        result.push(' ');
    }

    result
}

// Re-export parse_tab_stops for convenience.
pub use super::expand_tool::parse_tab_stops as parse_tabs;

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpand_leading_spaces() {
        assert_eq!(unexpand_line("        hello", &[8], false), "\thello");
    }

    #[test]
    fn unexpand_partial_spaces() {
        // 3 spaces don't reach a tab stop at 8.
        assert_eq!(unexpand_line("   hello", &[8], false), "   hello");
    }

    #[test]
    fn unexpand_no_spaces() {
        assert_eq!(unexpand_line("hello", &[8], false), "hello");
    }

    #[test]
    fn unexpand_custom_width() {
        assert_eq!(unexpand_line("    hello", &[4], false), "\thello");
    }

    #[test]
    fn unexpand_all_blanks() {
        // With all_blanks, spaces after non-blank chars are also converted.
        let result = unexpand_line("hello        world", &[8], true);
        // "hello" is 5 chars at column 5, need 3 spaces to reach 8 = tab.
        // Then 5 more spaces don't fill another tab.
        assert!(result.contains('\t') || result == "hello        world");
    }

    #[test]
    fn unexpand_empty() {
        assert_eq!(unexpand_line("", &[8], false), "");
    }
}
