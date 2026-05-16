//! FIND / SEARCH — substring lookup.
//!
//! Both functions return a 1-based *character* (not byte) position. They
//! differ in:
//!
//! | feature             | FIND          | SEARCH                 |
//! |---------------------|---------------|------------------------|
//! | case-sensitive      | yes           | no                     |
//! | wildcards (`*` `?`) | no            | yes                    |
//!
//! Both return `TextError::NotFound` (Excel `#VALUE!`) if the needle is
//! absent from the haystack starting at `start`.
//!
//! ## Wildcard glob matcher
//!
//! SEARCH's wildcards are *not* regex. They are:
//!
//! - `?` matches exactly one character (any character).
//! - `*` matches zero or more characters (any characters).
//! - Everything else matches itself (case-insensitively).
//!
//! No anchoring, no character classes, no escapes. Hand-rolled matcher with
//! backtracking on `*`. Returns the *char index* at which the match begins.

use crate::{iter_character, TextError};
use r_vector::Character;

/// `FIND(needle, haystack, [start])` — case-sensitive, no wildcards.
///
/// `start` is 1-based; defaults to 1. Returns the 1-based char position of
/// the first match, or `NotFound` if absent.
pub fn find(needle: &str, haystack: &str, start: Option<usize>) -> Result<usize, TextError> {
    let start = start.unwrap_or(1);
    if start < 1 {
        return Err(TextError::BadParameter {
            name: "start_num",
            value: start.to_string(),
        });
    }
    let start0 = start - 1;
    let hay: Vec<char> = haystack.chars().collect();
    if start0 > hay.len() {
        return Err(TextError::NotFound {
            function: "FIND",
            needle: needle.to_string(),
        });
    }
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() {
        // Excel: FIND("", ...) returns start.
        return Ok(start);
    }
    for i in start0..=hay.len().saturating_sub(needle_chars.len()) {
        if hay[i..i + needle_chars.len()] == needle_chars[..] {
            return Ok(i + 1);
        }
    }
    Err(TextError::NotFound {
        function: "FIND",
        needle: needle.to_string(),
    })
}

/// `SEARCH(needle, haystack, [start])` — case-insensitive; `*` and `?`
/// wildcards. Returns 1-based char position of the first match.
pub fn search(needle: &str, haystack: &str, start: Option<usize>) -> Result<usize, TextError> {
    let start = start.unwrap_or(1);
    if start < 1 {
        return Err(TextError::BadParameter {
            name: "start_num",
            value: start.to_string(),
        });
    }
    let start0 = start - 1;
    // Case-fold haystack and needle to lowercase for comparison.
    let hay: Vec<char> = haystack.to_lowercase().chars().collect();
    let pat: Vec<char> = needle.to_lowercase().chars().collect();

    if pat.is_empty() {
        return Ok(start);
    }
    if start0 > hay.len() {
        return Err(TextError::NotFound {
            function: "SEARCH",
            needle: needle.to_string(),
        });
    }
    for i in start0..=hay.len() {
        if glob_match_at(&pat, &hay, i) {
            return Ok(i + 1);
        }
    }
    Err(TextError::NotFound {
        function: "SEARCH",
        needle: needle.to_string(),
    })
}

/// Glob matcher. Returns true iff `pat` matches a prefix of `hay[start..]`
/// (or all of `hay[start..]` if the pattern ends without `*`). Supports `?`
/// (any one char) and `*` (any zero or more chars). Both `pat` and `hay`
/// are pre-lowercased.
///
/// We implement the classic two-pointer algorithm with a single backtrack
/// point: when `*` is encountered, remember the position in pattern and the
/// current position in the haystack; on mismatch later, rewind pattern to
/// just after the `*` and advance the haystack by one.
fn glob_match_at(pat: &[char], hay: &[char], start: usize) -> bool {
    let mut h = start;
    let mut p = 0usize;
    let mut star_p: Option<usize> = None;
    let mut star_h: usize = 0;

    while h <= hay.len() {
        if p < pat.len() && pat[p] == '*' {
            // Record backtrack point and advance past the `*`.
            star_p = Some(p);
            star_h = h;
            p += 1;
        } else if p < pat.len()
            && h < hay.len()
            && (pat[p] == '?' || pat[p] == hay[h])
        {
            p += 1;
            h += 1;
        } else if let Some(sp) = star_p {
            // Mismatch (or pattern exhausted but hay isn't): try matching
            // one more char with the last `*`.
            p = sp + 1;
            star_h += 1;
            h = star_h;
            if h > hay.len() {
                return false;
            }
        } else {
            return false;
        }

        // Once the pattern is exhausted, we've matched whatever the haystack
        // consumed: a search "starts at i" is satisfied as long as the
        // pattern is fully consumed, regardless of remaining haystack chars.
        if p == pat.len() {
            return true;
        }
    }
    // p == pat.len() handled inside loop. Reaching here means we ran out of
    // haystack with pattern unfinished.
    false
}

/// Vector `FIND`. NA in / NA out; not-found collapses to NA.
pub fn find_vec(needle: &str, haystacks: &Character, start: Option<usize>) -> Vec<Option<usize>> {
    iter_character(haystacks)
        .map(|cell| cell.and_then(|s| find(needle, s, start).ok()))
        .collect()
}

/// Vector `SEARCH`. NA in / NA out; not-found collapses to NA.
pub fn search_vec(needle: &str, haystacks: &Character, start: Option<usize>) -> Vec<Option<usize>> {
    iter_character(haystacks)
        .map(|cell| cell.and_then(|s| search(needle, s, start).ok()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_basic() {
        assert_eq!(find("world", "hello world", None).unwrap(), 7);
        assert_eq!(find("h", "hello", None).unwrap(), 1);
        assert_eq!(find("o", "hello world", None).unwrap(), 5);
    }

    #[test]
    fn find_is_case_sensitive() {
        assert!(find("WORLD", "hello world", None).is_err());
    }

    #[test]
    fn find_with_start() {
        // Skip the first "o"
        assert_eq!(find("o", "hello world", Some(6)).unwrap(), 8);
    }

    #[test]
    fn find_not_found() {
        let e = find("xyz", "hello", None).unwrap_err();
        assert!(matches!(e, TextError::NotFound { function: "FIND", .. }));
    }

    #[test]
    fn find_empty_needle() {
        assert_eq!(find("", "hello", None).unwrap(), 1);
        assert_eq!(find("", "hello", Some(3)).unwrap(), 3);
    }

    #[test]
    fn find_unicode_1based_chars() {
        // "漢" at char position 1, "字" at 2. FIND must NOT report byte
        // offsets.
        assert_eq!(find("字", "漢字日本", None).unwrap(), 2);
        assert_eq!(find("日本", "漢字日本", None).unwrap(), 3);
    }

    #[test]
    fn search_is_case_insensitive() {
        assert_eq!(search("WORLD", "hello world", None).unwrap(), 7);
        assert_eq!(search("HeLLo", "hello world", None).unwrap(), 1);
    }

    #[test]
    fn search_wildcard_question() {
        // ? matches exactly one char
        assert_eq!(search("h?llo", "hello", None).unwrap(), 1);
        assert_eq!(search("h?l", "well hello", None).unwrap(), 6);
        assert!(search("h??llo", "hello", None).is_err()); // too many ?
    }

    #[test]
    fn search_wildcard_star() {
        // * matches zero or more
        assert_eq!(search("h*o", "hello", None).unwrap(), 1);
        assert_eq!(search("h*o", "ho", None).unwrap(), 1);
        assert_eq!(search("*world", "hello world", None).unwrap(), 1);
        assert_eq!(search("hello*", "hello world", None).unwrap(), 1);
    }

    #[test]
    fn search_combined_wildcards() {
        assert_eq!(search("h?l*o", "hello world", None).unwrap(), 1);
        assert_eq!(search("?or*", "hello world", None).unwrap(), 7);
    }

    #[test]
    fn search_not_found() {
        assert!(search("xyz*", "hello", None).is_err());
    }

    #[test]
    fn search_with_start() {
        // Two "hellos"; second one starts at char position 7 (counting space).
        assert_eq!(search("hello", "hello hello", Some(3)).unwrap(), 7);
    }

    #[test]
    fn search_unicode() {
        assert_eq!(search("字", "漢字日本", None).unwrap(), 2);
    }

    #[test]
    fn find_vec_propagates_na_and_misses() {
        let hay = Character::from_options(vec![
            Some("hello world".into()),
            None,
            Some("foo".into()),
        ]);
        let got = find_vec("o", &hay, None);
        assert_eq!(got, vec![Some(5), None, Some(2)]);
    }

    #[test]
    fn search_vec_propagates_na() {
        let hay = Character::from_options(vec![Some("Hello".into()), None]);
        let got = search_vec("h*o", &hay, None);
        assert_eq!(got, vec![Some(1), None]);
    }

    #[test]
    fn find_start_past_end_is_not_found() {
        assert!(find("x", "abc", Some(100)).is_err());
    }
}
