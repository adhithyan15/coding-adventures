//! # tr — Translate or Delete Characters
//!
//! This module implements the business logic for the `tr` command.
//! tr operates on individual characters, translating, deleting, or
//! squeezing them according to two character set specifications.
//!
//! ## Operations
//!
//! ```text
//!     tr 'a-z' 'A-Z'         Translate lowercase to uppercase
//!     tr -d 'aeiou'           Delete all vowels
//!     tr -s 'a-z'             Squeeze repeated lowercase chars
//!     tr -c 'a-z' '_'         Replace non-lowercase with '_'
//! ```
//!
//! ## Character Sets and Ranges
//!
//! tr supports character ranges like `a-z` (all lowercase letters)
//! and `0-9` (all digits). Characters in SET1 are mapped positionally
//! to characters in SET2.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Expand a character set specification into individual characters.
///
/// Handles range notation like "a-z" by expanding it to all characters
/// in that range. For example: "a-d" becomes "abcd".
pub fn expand_set(set: &str) -> Vec<char> {
    let chars: Vec<char> = set.chars().collect();
    let mut result: Vec<char> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        // Check for range notation: "a-z"
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            let start = chars[i];
            let end = chars[i + 2];
            if start <= end {
                for c in start..=end {
                    result.push(c);
                }
            } else {
                // Reverse range — treat as literals.
                result.push(chars[i]);
                result.push(chars[i + 1]);
                result.push(chars[i + 2]);
            }
            i += 3;
        } else if chars[i] == '\\' && i + 1 < chars.len() {
            // Handle escape sequences.
            match chars[i + 1] {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '\\' => result.push('\\'),
                other => result.push(other),
            }
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Translate characters: replace each char in SET1 with the corresponding
/// char in SET2.
///
/// If SET2 is shorter than SET1, the last character of SET2 is used for
/// the remaining mappings.
pub fn translate(content: &str, set1: &str, set2: &str, complement: bool) -> String {
    let exp1 = expand_set(set1);
    let exp2 = expand_set(set2);

    content
        .chars()
        .map(|ch| {
            let in_set = exp1.contains(&ch);
            let should_translate = if complement { !in_set } else { in_set };

            if should_translate && !exp2.is_empty() {
                if complement {
                    exp2[0]
                } else if let Some(idx) = exp1.iter().position(|&c| c == ch) {
                    if idx < exp2.len() {
                        exp2[idx]
                    } else {
                        *exp2.last().unwrap()
                    }
                } else {
                    ch
                }
            } else {
                ch
            }
        })
        .collect()
}

/// Delete characters in SET1 from the content.
pub fn delete_chars(content: &str, set1: &str, complement: bool) -> String {
    let expanded = expand_set(set1);

    content
        .chars()
        .filter(|ch| {
            let in_set = expanded.contains(ch);
            if complement { in_set } else { !in_set }
        })
        .collect()
}

/// Squeeze adjacent repeated characters that appear in the set.
///
/// Replaces runs of the same character with a single occurrence,
/// but only for characters that are in the specified set.
pub fn squeeze_repeats(content: &str, set: &str) -> String {
    let expanded = expand_set(set);
    let mut result = String::new();
    let mut last_char: Option<char> = None;

    for ch in content.chars() {
        if let Some(prev) = last_char {
            if ch == prev && expanded.contains(&ch) {
                continue; // Skip repeated character
            }
        }
        result.push(ch);
        last_char = Some(ch);
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
    fn expand_range() {
        let result = expand_set("a-d");
        assert_eq!(result, vec!['a', 'b', 'c', 'd']);
    }

    #[test]
    fn expand_literal() {
        let result = expand_set("abc");
        assert_eq!(result, vec!['a', 'b', 'c']);
    }

    #[test]
    fn expand_mixed() {
        let result = expand_set("a-c0-2");
        assert_eq!(result, vec!['a', 'b', 'c', '0', '1', '2']);
    }

    #[test]
    fn translate_simple() {
        assert_eq!(translate("hello", "helo", "HELO", false), "HELLO");
    }

    #[test]
    fn translate_range() {
        let result = translate("abc", "a-c", "A-C", false);
        assert_eq!(result, "ABC");
    }

    #[test]
    fn delete_vowels() {
        assert_eq!(delete_chars("hello world", "aeiou", false), "hll wrld");
    }

    #[test]
    fn delete_complement() {
        // Delete everything that is NOT a vowel.
        assert_eq!(delete_chars("hello", "helo", true), "");
    }

    #[test]
    fn squeeze_simple() {
        assert_eq!(squeeze_repeats("aabbcc", "a-c"), "abc");
    }

    #[test]
    fn squeeze_partial() {
        // Only squeeze 'a', not 'b'.
        assert_eq!(squeeze_repeats("aabb", "a"), "abb");
    }

    #[test]
    fn empty_input() {
        assert_eq!(translate("", "a", "b", false), "");
        assert_eq!(delete_chars("", "a", false), "");
        assert_eq!(squeeze_repeats("", "a"), "");
    }
}
