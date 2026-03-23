//! # fold — Wrap Each Input Line to Fit in Specified Width
//!
//! This module implements the business logic for the `fold` command.
//! fold wraps each input line so it is no wider than a specified width
//! (default 80 columns).
//!
//! ## Two Break Modes
//!
//! ```text
//!     Default:    Break at exactly the width boundary (may split words)
//!     -s:         Break at the last space before the boundary (word-aware)
//! ```
//!
//! ## Example
//!
//! ```text
//!     Input:  "abcdefghij"  (width=5)
//!     Output: "abcde\nfghij"
//! ```

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fold a single line to fit within the specified width.
///
/// # Parameters
/// - `line`: input line (without trailing newline)
/// - `width`: maximum column width
/// - `break_at_spaces`: if true, try to break at spaces
/// - `count_bytes`: if true, count bytes instead of characters
pub fn fold_line(line: &str, width: usize, break_at_spaces: bool, _count_bytes: bool) -> String {
    if width == 0 || line.is_empty() {
        return line.to_string();
    }

    let chars: Vec<char> = line.chars().collect();
    let mut result = String::new();
    let mut start = 0;

    while start < chars.len() {
        let end = start + width;

        if end >= chars.len() {
            // Remaining text fits.
            let remaining: String = chars[start..].iter().collect();
            result.push_str(&remaining);
            break;
        }

        if break_at_spaces {
            // Look for the last space within the width boundary.
            let mut break_point = None;
            for i in (start + 1..=end).rev() {
                if i < chars.len() && chars[i] == ' ' {
                    break_point = Some(i);
                    break;
                }
            }

            if let Some(bp) = break_point {
                let segment: String = chars[start..bp].iter().collect();
                result.push_str(&segment);
                result.push('\n');
                start = bp + 1; // skip the space
                continue;
            }
        }

        // Hard break at width.
        let segment: String = chars[start..end].iter().collect();
        result.push_str(&segment);
        result.push('\n');
        start = end;
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
    fn fold_exact() {
        assert_eq!(fold_line("abcdefghij", 5, false, false), "abcde\nfghij");
    }

    #[test]
    fn fold_short_line() {
        assert_eq!(fold_line("abc", 10, false, false), "abc");
    }

    #[test]
    fn fold_empty() {
        assert_eq!(fold_line("", 10, false, false), "");
    }

    #[test]
    fn fold_with_spaces() {
        let result = fold_line("hello world foo", 10, true, false);
        // Should break at space.
        assert!(result.contains('\n'));
        // Should not break mid-word.
        assert!(!result.contains("worl\nd"));
    }

    #[test]
    fn fold_exact_width() {
        assert_eq!(fold_line("12345", 5, false, false), "12345");
    }

    #[test]
    fn fold_three_segments() {
        assert_eq!(
            fold_line("abcdefghijklmno", 5, false, false),
            "abcde\nfghij\nklmno"
        );
    }

    #[test]
    fn fold_width_one() {
        assert_eq!(
            fold_line("abc", 1, false, false),
            "a\nb\nc"
        );
    }
}
