//! LEFT / RIGHT / MID — substring extraction.
//!
//! All three operate on **Unicode scalar values** (Rust `char`s), not bytes.
//! Positions are **1-based** to match the Excel / VisiCalc API.
//!
//! | function          | semantics                                          |
//! |-------------------|----------------------------------------------------|
//! | `LEFT(s, n)`      | first `n` chars; if `n > len(s)`, returns `s`      |
//! | `RIGHT(s, n)`     | last `n` chars; if `n > len(s)`, returns `s`       |
//! | `MID(s, p, n)`    | `n` chars starting at 1-based position `p`         |
//!
//! Edge cases (Excel parity):
//!
//! - `LEFT("abc", 0) == ""`; `RIGHT("abc", 0) == ""`.
//! - `LEFT("abc", -1)` → `TextError::BadParameter` (Excel `#VALUE!`).
//! - `MID("abc", 0, ...)` → `BadParameter`. Excel disallows `start_num < 1`.
//! - `MID("abc", 5, 2) == ""` — start past end yields empty.
//! - `MID("abc", 2, 10) == "bc"` — length clamps to what's available.

use crate::{iter_character, TextError};
use r_vector::Character;

/// `LEFT(text, n)` — first `n` characters (Unicode scalars).
///
/// Returns `BadParameter` if `n < 0`.
pub fn left(s: &str, n: i64) -> Result<String, TextError> {
    if n < 0 {
        return Err(TextError::BadParameter {
            name: "num_chars",
            value: n.to_string(),
        });
    }
    Ok(s.chars().take(n as usize).collect())
}

/// `RIGHT(text, n)` — last `n` characters (Unicode scalars).
///
/// Returns `BadParameter` if `n < 0`.
pub fn right(s: &str, n: i64) -> Result<String, TextError> {
    if n < 0 {
        return Err(TextError::BadParameter {
            name: "num_chars",
            value: n.to_string(),
        });
    }
    let n = n as usize;
    let total = s.chars().count();
    if n >= total {
        return Ok(s.to_string());
    }
    // Skip total - n chars from the front.
    Ok(s.chars().skip(total - n).collect())
}

/// `MID(text, start, length)` — `length` characters starting at 1-based
/// `start`.
///
/// - `start < 1` → `BadParameter`
/// - `length < 0` → `BadParameter`
/// - `start > len(s)` → empty string
/// - `length` longer than remaining chars → clamps
pub fn mid(s: &str, start: i64, length: i64) -> Result<String, TextError> {
    if start < 1 {
        return Err(TextError::BadParameter {
            name: "start_num",
            value: start.to_string(),
        });
    }
    if length < 0 {
        return Err(TextError::BadParameter {
            name: "num_chars",
            value: length.to_string(),
        });
    }
    let start0 = (start - 1) as usize;
    Ok(s.chars().skip(start0).take(length as usize).collect())
}

/// Vector `LEFT`. NA in / NA out; errors collapse to NA.
pub fn left_vec(x: &Character, n: i64) -> Character {
    let out: Vec<Option<String>> = iter_character(x)
        .map(|cell| cell.and_then(|s| left(s, n).ok()))
        .collect();
    Character::from_options(out)
}

/// Vector `RIGHT`. NA in / NA out; errors collapse to NA.
pub fn right_vec(x: &Character, n: i64) -> Character {
    let out: Vec<Option<String>> = iter_character(x)
        .map(|cell| cell.and_then(|s| right(s, n).ok()))
        .collect();
    Character::from_options(out)
}

/// Vector `MID`. NA in / NA out; errors collapse to NA.
pub fn mid_vec(x: &Character, start: i64, length: i64) -> Character {
    let out: Vec<Option<String>> = iter_character(x)
        .map(|cell| cell.and_then(|s| mid(s, start, length).ok()))
        .collect();
    Character::from_options(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::Vector;

    #[test]
    fn left_basic() {
        assert_eq!(left("hello", 3).unwrap(), "hel");
        assert_eq!(left("hello", 0).unwrap(), "");
        assert_eq!(left("hello", 99).unwrap(), "hello");
        assert_eq!(left("", 3).unwrap(), "");
    }

    #[test]
    fn left_unicode() {
        assert_eq!(left("héllo", 2).unwrap(), "hé");
        assert_eq!(left("漢字日本", 2).unwrap(), "漢字");
        assert_eq!(left("🙂🎉a", 2).unwrap(), "🙂🎉");
    }

    #[test]
    fn left_negative_errors() {
        let e = left("hi", -1).unwrap_err();
        assert!(matches!(e, TextError::BadParameter { name: "num_chars", .. }));
    }

    #[test]
    fn right_basic() {
        assert_eq!(right("hello", 3).unwrap(), "llo");
        assert_eq!(right("hello", 0).unwrap(), "");
        assert_eq!(right("hello", 99).unwrap(), "hello");
    }

    #[test]
    fn right_unicode() {
        assert_eq!(right("héllo", 3).unwrap(), "llo");
        assert_eq!(right("漢字日本", 2).unwrap(), "日本");
    }

    #[test]
    fn right_negative_errors() {
        assert!(right("hi", -1).is_err());
    }

    #[test]
    fn mid_basic() {
        assert_eq!(mid("hello", 1, 3).unwrap(), "hel");
        assert_eq!(mid("hello", 2, 3).unwrap(), "ell");
        assert_eq!(mid("hello", 5, 1).unwrap(), "o");
        // start past end
        assert_eq!(mid("hello", 100, 3).unwrap(), "");
        // length past end clamps
        assert_eq!(mid("hello", 3, 100).unwrap(), "llo");
        // zero length
        assert_eq!(mid("hello", 3, 0).unwrap(), "");
    }

    #[test]
    fn mid_unicode() {
        assert_eq!(mid("漢字日本", 2, 2).unwrap(), "字日");
        assert_eq!(mid("a🙂b", 2, 1).unwrap(), "🙂");
    }

    #[test]
    fn mid_bad_start() {
        assert!(mid("hi", 0, 1).is_err());
        assert!(mid("hi", -3, 1).is_err());
    }

    #[test]
    fn mid_bad_length() {
        assert!(mid("hi", 1, -1).is_err());
    }

    #[test]
    fn vec_variants_propagate_na() {
        let x = Character::from_options(vec![
            Some("hello".into()),
            None,
            Some("漢字".into()),
        ]);
        let got = left_vec(&x, 2);
        assert_eq!(got.get(0), Some(&Some("he".into())));
        assert_eq!(got.get(1), Some(&None));
        assert_eq!(got.get(2), Some(&Some("漢字".into())));

        let got = right_vec(&x, 2);
        assert_eq!(got.get(0), Some(&Some("lo".into())));
        assert!(got.is_na(1));

        let got = mid_vec(&x, 2, 2);
        assert_eq!(got.get(0), Some(&Some("el".into())));
        assert!(got.is_na(1));
    }

    #[test]
    fn vec_variants_collapse_errors_to_na() {
        let x = Character::from_strings(["hi"]);
        let got = left_vec(&x, -1);
        assert!(got.is_na(0));
    }
}
