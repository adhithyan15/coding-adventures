//! TEXTSPLIT / TEXTBEFORE / TEXTAFTER — text segmentation.
//!
//! These mirror Excel 365's split family:
//!
//! - `TEXTSPLIT(text, col_delim)` — split into substrings on each occurrence
//!   of `col_delim`. (We expose the column-split flavour only; row-splitting
//!   is the caller's job since we don't model 2-D shapes here.)
//! - `TEXTBEFORE(text, delim, [instance])` — substring before the `instance`-th
//!   occurrence of `delim`. Instance defaults to 1. Negative instance counts
//!   from the right (Excel parity).
//! - `TEXTAFTER(text, delim, [instance])` — substring after the `instance`-th
//!   occurrence.
//!
//! If `delim` is empty, all three return `BadParameter`. If the requested
//! `instance` doesn't exist, `TEXTBEFORE` / `TEXTAFTER` return `NotFound`.

use crate::TextError;
use r_vector::Character;

/// `TEXTSPLIT(text, delim)`. Splits on every occurrence of `delim`. Empty
/// `delim` yields `BadParameter`.
pub fn textsplit(text: &str, delim: &str) -> Result<Character, TextError> {
    if delim.is_empty() {
        return Err(TextError::BadParameter {
            name: "delimiter",
            value: String::new(),
        });
    }
    let pieces: Vec<Option<String>> = text.split(delim).map(|p| Some(p.to_string())).collect();
    Ok(Character::from_options(pieces))
}

/// `TEXTBEFORE(text, delim, [instance])`.
///
/// `instance` is 1-based. A negative `instance` counts from the right
/// (e.g. -1 means the last occurrence). `instance == 0` is `BadParameter`.
pub fn textbefore(text: &str, delim: &str, instance: i64) -> Result<String, TextError> {
    if delim.is_empty() {
        return Err(TextError::BadParameter {
            name: "delimiter",
            value: String::new(),
        });
    }
    if instance == 0 {
        return Err(TextError::BadParameter {
            name: "instance_num",
            value: "0".to_string(),
        });
    }
    let positions = find_all(text, delim);
    if positions.is_empty() {
        return Err(TextError::NotFound {
            function: "TEXTBEFORE",
            needle: delim.to_string(),
        });
    }
    let idx = resolve_instance(instance, positions.len())?;
    let cut = positions[idx];
    Ok(text[..cut].to_string())
}

/// `TEXTAFTER(text, delim, [instance])`. See `textbefore` for instance
/// semantics.
pub fn textafter(text: &str, delim: &str, instance: i64) -> Result<String, TextError> {
    if delim.is_empty() {
        return Err(TextError::BadParameter {
            name: "delimiter",
            value: String::new(),
        });
    }
    if instance == 0 {
        return Err(TextError::BadParameter {
            name: "instance_num",
            value: "0".to_string(),
        });
    }
    let positions = find_all(text, delim);
    if positions.is_empty() {
        return Err(TextError::NotFound {
            function: "TEXTAFTER",
            needle: delim.to_string(),
        });
    }
    let idx = resolve_instance(instance, positions.len())?;
    let cut = positions[idx] + delim.len();
    Ok(text[cut..].to_string())
}

/// Return the byte offsets of every non-overlapping occurrence of `needle`
/// in `haystack`.
fn find_all(haystack: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = haystack[i..].find(needle) {
        let abs = i + rel;
        out.push(abs);
        i = abs + needle.len();
    }
    out
}

/// Translate a 1-based `instance` (possibly negative) into a 0-based index
/// into a list of `total` positions.
fn resolve_instance(instance: i64, total: usize) -> Result<usize, TextError> {
    if instance > 0 {
        let idx = (instance - 1) as usize;
        if idx >= total {
            return Err(TextError::NotFound {
                function: "TEXTBEFORE/AFTER",
                needle: format!("instance {instance}"),
            });
        }
        Ok(idx)
    } else {
        // -1 = last, -2 = second-to-last, ...
        let from_end = (-instance) as usize;
        if from_end > total {
            return Err(TextError::NotFound {
                function: "TEXTBEFORE/AFTER",
                needle: format!("instance {instance}"),
            });
        }
        Ok(total - from_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::Vector;

    #[test]
    fn textsplit_basic() {
        let parts = textsplit("a,b,c", ",").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts.get(0), Some(&Some("a".into())));
        assert_eq!(parts.get(1), Some(&Some("b".into())));
        assert_eq!(parts.get(2), Some(&Some("c".into())));
    }

    #[test]
    fn textsplit_keeps_empty_segments() {
        let parts = textsplit("a,,b", ",").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts.get(1), Some(&Some("".into())));
    }

    #[test]
    fn textsplit_multichar_delim() {
        let parts = textsplit("aXXbXXc", "XX").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts.get(1), Some(&Some("b".into())));
    }

    #[test]
    fn textsplit_no_match() {
        // No delim found: returns the whole string as a single element.
        let parts = textsplit("hello", ",").unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts.get(0), Some(&Some("hello".into())));
    }

    #[test]
    fn textsplit_empty_delim_errors() {
        assert!(textsplit("abc", "").is_err());
    }

    #[test]
    fn textbefore_positive_instance() {
        assert_eq!(textbefore("a-b-c-d", "-", 1).unwrap(), "a");
        assert_eq!(textbefore("a-b-c-d", "-", 2).unwrap(), "a-b");
        assert_eq!(textbefore("a-b-c-d", "-", 3).unwrap(), "a-b-c");
    }

    #[test]
    fn textbefore_negative_instance() {
        // -1 == last delim
        assert_eq!(textbefore("a-b-c-d", "-", -1).unwrap(), "a-b-c");
        assert_eq!(textbefore("a-b-c-d", "-", -2).unwrap(), "a-b");
    }

    #[test]
    fn textbefore_missing() {
        assert!(textbefore("abc", "-", 1).is_err());
        assert!(textbefore("a-b", "-", 5).is_err());
        assert!(textbefore("a-b", "-", -5).is_err());
    }

    #[test]
    fn textbefore_instance_zero_errors() {
        assert!(textbefore("a-b", "-", 0).is_err());
    }

    #[test]
    fn textafter_positive_instance() {
        assert_eq!(textafter("a-b-c-d", "-", 1).unwrap(), "b-c-d");
        assert_eq!(textafter("a-b-c-d", "-", 3).unwrap(), "d");
    }

    #[test]
    fn textafter_negative_instance() {
        assert_eq!(textafter("a-b-c-d", "-", -1).unwrap(), "d");
        assert_eq!(textafter("a-b-c-d", "-", -2).unwrap(), "c-d");
    }

    #[test]
    fn textafter_missing() {
        assert!(textafter("abc", "-", 1).is_err());
        assert!(textafter("a-b", "-", 0).is_err());
    }

    #[test]
    fn textsplit_unicode() {
        let parts = textsplit("漢-字-日", "-").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts.get(0), Some(&Some("漢".into())));
        assert_eq!(parts.get(2), Some(&Some("日".into())));
    }

    #[test]
    fn textbefore_unicode_delim() {
        assert_eq!(textbefore("aXb", "X", 1).unwrap(), "a");
        assert_eq!(textbefore("a→b→c", "→", 1).unwrap(), "a");
        assert_eq!(textafter("a→b→c", "→", 1).unwrap(), "b→c");
    }
}
