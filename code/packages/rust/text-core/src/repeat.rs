//! REPT — string repetition.
//!
//! `REPT(text, count)` repeats `text` `count` times. `count == 0` returns an
//! empty string; negative `count` is `BadParameter`. Excel additionally caps
//! the total output length at 32,767 characters; we enforce the same cap
//! since spreadsheets that consume our output expect it. Larger requests
//! return `BadParameter`.

use crate::{iter_character, TextError};
use r_vector::Character;

/// Excel's cell-content length cap. Used as the REPT output ceiling.
pub const REPT_MAX_LEN: usize = 32_767;

/// `REPT(text, count)`.
pub fn rept(text: &str, count: i64) -> Result<String, TextError> {
    if count < 0 {
        return Err(TextError::BadParameter {
            name: "number_times",
            value: count.to_string(),
        });
    }
    let count = count as usize;
    // Compute final size in *chars* for the cap. (Excel uses chars, not bytes.)
    let unit_chars = text.chars().count();
    let total_chars = unit_chars.saturating_mul(count);
    if total_chars > REPT_MAX_LEN {
        return Err(TextError::BadParameter {
            name: "number_times",
            value: format!("would exceed {REPT_MAX_LEN} chars"),
        });
    }
    Ok(text.repeat(count))
}

/// Vector `REPT`. NA in / NA out; errors collapse to NA.
pub fn rept_vec(x: &Character, count: i64) -> Character {
    let out: Vec<Option<String>> = iter_character(x)
        .map(|cell| cell.and_then(|s| rept(s, count).ok()))
        .collect();
    Character::from_options(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::Vector;

    #[test]
    fn rept_basic() {
        assert_eq!(rept("ab", 3).unwrap(), "ababab");
        assert_eq!(rept("x", 0).unwrap(), "");
        assert_eq!(rept("", 5).unwrap(), "");
    }

    #[test]
    fn rept_unicode() {
        assert_eq!(rept("漢", 3).unwrap(), "漢漢漢");
        assert_eq!(rept("🙂", 2).unwrap(), "🙂🙂");
    }

    #[test]
    fn rept_negative_errors() {
        assert!(rept("a", -1).is_err());
    }

    #[test]
    fn rept_overflow_cap() {
        // 100 chars * 1000 = 100,000 > REPT_MAX_LEN
        let s = "x".repeat(100);
        assert!(rept(&s, 1000).is_err());
    }

    #[test]
    fn rept_vec_propagates_na() {
        let x = Character::from_options(vec![Some("a".into()), None]);
        let r = rept_vec(&x, 3);
        assert_eq!(r.get(0), Some(&Some("aaa".into())));
        assert!(r.is_na(1));
    }

    #[test]
    fn rept_vec_collapses_errors_to_na() {
        let x = Character::from_strings(["a"]);
        let r = rept_vec(&x, -1);
        assert!(r.is_na(0));
    }
}
