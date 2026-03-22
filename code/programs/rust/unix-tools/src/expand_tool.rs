//! # expand — Convert Tabs to Spaces
//!
//! This module implements the business logic for the `expand` command.
//! expand converts tab characters to the appropriate number of spaces,
//! maintaining column alignment.
//!
//! ## How Tab Expansion Works
//!
//! A tab doesn't always represent the same number of spaces. It advances
//! the cursor to the NEXT tab stop:
//!
//! ```text
//!     Tab stops every 8 columns:
//!     Column: 0  1  2  3  4  5  6  7  8
//!
//!     Tab at column 0 => 8 spaces (advance to 8)
//!     Tab at column 3 => 5 spaces (advance to 8)
//!     Tab at column 7 => 1 space  (advance to 8)
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a tab stop specification string into a list of tab stops.
///
/// The string can be:
/// - A single number: "4" (tab stops every 4 columns)
/// - A comma-separated list: "2,4,8" (tab stops at those columns)
pub fn parse_tab_stops(spec: &str) -> Result<Vec<usize>, String> {
    if spec.is_empty() {
        return Ok(vec![8]);
    }

    let stops: Result<Vec<usize>, _> = spec
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<usize>()
                .map_err(|_| format!("invalid tab stop: '{}'", s))
        })
        .collect();

    let stops = stops?;
    if stops.iter().any(|&s| s == 0) {
        return Err("tab stop must be positive".to_string());
    }
    Ok(stops)
}

/// Calculate the number of spaces needed to reach the next tab stop.
pub fn next_tab_stop(column: usize, tab_stops: &[usize]) -> usize {
    if tab_stops.len() == 1 {
        // Regular interval.
        let interval = tab_stops[0];
        interval - (column % interval)
    } else {
        // Variable tab stops: find the first stop past the current column.
        for &stop in tab_stops {
            if stop > column {
                return stop - column;
            }
        }
        // Past all defined stops — use 1 as fallback.
        1
    }
}

/// Expand tabs in a single line to spaces.
///
/// # Parameters
/// - `line`: input line (without newline)
/// - `tab_stops`: parsed tab stop positions
/// - `initial_only`: if true, only expand tabs before the first non-blank
pub fn expand_line(line: &str, tab_stops: &[usize], initial_only: bool) -> String {
    let mut result = String::new();
    let mut column = 0;
    let mut seen_non_blank = false;

    for ch in line.chars() {
        if ch == '\t' {
            if initial_only && seen_non_blank {
                result.push('\t');
                column += 1;
            } else {
                let spaces = next_tab_stop(column, tab_stops);
                for _ in 0..spaces {
                    result.push(' ');
                }
                column += spaces;
            }
        } else {
            if ch != ' ' {
                seen_non_blank = true;
            }
            result.push(ch);
            column += 1;
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
    fn expand_tab_at_start() {
        assert_eq!(expand_line("\thello", &[8], false), "        hello");
    }

    #[test]
    fn expand_tab_mid_line() {
        // "abc" = 3 chars, tab needs 5 spaces to reach column 8.
        assert_eq!(expand_line("abc\tdef", &[8], false), "abc     def");
    }

    #[test]
    fn expand_custom_width() {
        assert_eq!(expand_line("\thello", &[4], false), "    hello");
    }

    #[test]
    fn expand_initial_only() {
        let result = expand_line("\thello\tworld", &[4], true);
        assert_eq!(result, "    hello\tworld");
    }

    #[test]
    fn no_tabs() {
        assert_eq!(expand_line("hello world", &[8], false), "hello world");
    }

    #[test]
    fn parse_default() {
        assert_eq!(parse_tab_stops(""), Ok(vec![8]));
    }

    #[test]
    fn parse_custom() {
        assert_eq!(parse_tab_stops("4"), Ok(vec![4]));
    }

    #[test]
    fn parse_multiple() {
        assert_eq!(parse_tab_stops("2,4,8"), Ok(vec![2, 4, 8]));
    }

    #[test]
    fn next_tab_regular() {
        assert_eq!(next_tab_stop(0, &[8]), 8);
        assert_eq!(next_tab_stop(3, &[8]), 5);
        assert_eq!(next_tab_stop(7, &[8]), 1);
        assert_eq!(next_tab_stop(8, &[8]), 8);
    }
}
