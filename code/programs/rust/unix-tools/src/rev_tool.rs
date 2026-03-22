//! # rev — Reverse Lines Characterwise
//!
//! This module implements the business logic for the `rev` command.
//! The `rev` utility reverses each line of input character by character.
//! It does NOT reverse the order of lines — each line is individually
//! reversed in place.
//!
//! ## How It Works
//!
//! ```text
//!     Input:     "hello\nworld\n"
//!
//!     Line 1:    "hello"  →  "olleh"
//!     Line 2:    "world"  →  "dlrow"
//!
//!     Output:    "olleh\ndlrow\n"
//! ```
//!
//! ## Unicode Awareness
//!
//! We reverse by Unicode characters (graphemes), not bytes. This means
//! multi-byte UTF-8 characters stay intact:
//!
//! ```text
//!     Input:     "cafe\u{0301}"   (café with combining accent)
//!     Reversed:  "\u{0301}efac"   (character-level reversal)
//! ```
//!
//! Note: We reverse by `char`, not by grapheme cluster. This matches
//! the behavior of most `rev` implementations.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Reverse each line in the content, character by character.
///
/// Lines are delimited by newline characters. Each line is reversed
/// independently — the order of lines in the output matches the
/// order in the input.
///
/// # Algorithm
///
/// 1. Split the content into lines (preserving the newline structure)
/// 2. For each line, reverse the characters
/// 3. Rejoin with newlines
///
/// ```text
///     "abc\ndefg\n"
///       ↓ split
///     ["abc", "defg", ""]
///       ↓ reverse each
///     ["cba", "gfed", ""]
///       ↓ rejoin
///     "cba\ngfed\n"
/// ```
pub fn reverse_lines(content: &str) -> String {
    let has_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<&str> = content.split('\n').collect();

    // Remove phantom trailing empty element from split
    if has_trailing_newline && lines.last() == Some(&"") {
        lines.pop();
    }

    let reversed: Vec<String> = lines
        .iter()
        .map(|line| line.chars().rev().collect::<String>())
        .collect();

    let mut result = reversed.join("\n");
    if has_trailing_newline {
        result.push('\n');
    } else if !content.is_empty() && !has_trailing_newline {
        // No trailing newline in input — don't add one
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
    fn simple_reversal() {
        assert_eq!(reverse_lines("hello\n"), "olleh\n");
    }

    #[test]
    fn multiple_lines() {
        assert_eq!(reverse_lines("hello\nworld\n"), "olleh\ndlrow\n");
    }

    #[test]
    fn empty_content() {
        assert_eq!(reverse_lines(""), "");
    }

    #[test]
    fn single_character() {
        assert_eq!(reverse_lines("a\n"), "a\n");
    }

    #[test]
    fn palindrome() {
        assert_eq!(reverse_lines("racecar\n"), "racecar\n");
    }

    #[test]
    fn no_trailing_newline() {
        assert_eq!(reverse_lines("hello"), "olleh");
    }

    #[test]
    fn blank_lines() {
        assert_eq!(reverse_lines("abc\n\ndef\n"), "cba\n\nfed\n");
    }

    #[test]
    fn spaces_reversed() {
        assert_eq!(reverse_lines("a b c\n"), "c b a\n");
    }

    #[test]
    fn tabs_reversed() {
        assert_eq!(reverse_lines("a\tb\n"), "b\ta\n");
    }

    #[test]
    fn unicode_characters() {
        // Each character is reversed individually
        assert_eq!(reverse_lines("\u{00e9}ab\n"), "ba\u{00e9}\n");
    }
}
