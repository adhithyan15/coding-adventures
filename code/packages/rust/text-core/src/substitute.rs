//! SUBSTITUTE / REPLACE — string substitution.
//!
//! ## SUBSTITUTE
//!
//! `SUBSTITUTE(text, old, new, [instance])`:
//!
//! - If `instance` is `None`, every occurrence of `old` is replaced with
//!   `new`. (Equivalent to R's `gsub` with a fixed pattern.)
//! - If `instance` is `Some(k)`, only the *k*-th occurrence (1-based) is
//!   replaced; if there are fewer than `k` occurrences the original is
//!   returned unchanged.
//! - If `old` is empty, the input is returned unchanged. This matches Excel:
//!   substituting the empty string is a no-op (it never recurses infinitely).
//!
//! ## REPLACE
//!
//! `REPLACE(text, start, length, new)`:
//!
//! - 1-based `start`. Excel allows `start_num > len(text)`, in which case the
//!   new text is appended. We do the same: out-of-range `start` clamps to the
//!   end and inserts `new` there.
//! - `start < 1` → `BadParameter`.
//! - `length < 0` → `BadParameter`.
//! - Operates on **Unicode scalar values** (chars).

use crate::{iter_character, TextError};
use r_vector::Character;

/// `SUBSTITUTE(text, old, new, [instance])`.
pub fn substitute(text: &str, old: &str, new: &str, instance: Option<usize>) -> String {
    if old.is_empty() {
        // Excel: substituting the empty string is a no-op.
        return text.to_string();
    }
    match instance {
        None => text.replace(old, new),
        Some(k) if k == 0 => {
            // 1-based; instance 0 is meaningless. Match Excel's behaviour of
            // returning text unchanged (Excel returns #VALUE!, but in the
            // typical Rust API style we treat 0 as "no match wanted").
            // Callers wanting strict Excel behaviour can validate at the
            // boundary.
            text.to_string()
        }
        Some(k) => {
            // Find the kth occurrence and replace only that.
            let mut out = String::with_capacity(text.len());
            let mut seen = 0usize;
            // We walk byte-by-byte but check substring matches via
            // `text[i..].starts_with(old)` to handle UTF-8 safely (any byte
            // index where a multi-byte char starts is a char boundary).
            let bytes = text.as_bytes();
            let mut i = 0usize;
            while i < bytes.len() {
                if text.is_char_boundary(i) && text[i..].starts_with(old) {
                    seen += 1;
                    if seen == k {
                        out.push_str(new);
                        i += old.len();
                        // Copy the rest verbatim.
                        out.push_str(&text[i..]);
                        return out;
                    } else {
                        // Not the right instance: copy this old as-is.
                        out.push_str(&text[i..i + old.len()]);
                        i += old.len();
                        continue;
                    }
                }
                // Push a single character.
                // Find the next char boundary so we don't break UTF-8.
                let mut j = i + 1;
                while j <= bytes.len() && !text.is_char_boundary(j) {
                    j += 1;
                }
                out.push_str(&text[i..j]);
                i = j;
            }
            out
        }
    }
}

/// `REPLACE(text, start, length, new)`.
///
/// `start` is 1-based, in **chars**, not bytes.
pub fn replace(text: &str, start: i64, length: i64, new: &str) -> Result<String, TextError> {
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
    let length = length as usize;

    // Walk char-by-char accumulating the prefix, drop `length` chars, then
    // emit `new` and the suffix.
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    let head_end = start0.min(total);
    let drop_end = (start0 + length).min(total);

    let mut out = String::with_capacity(text.len() + new.len());
    out.extend(&chars[..head_end]);
    out.push_str(new);
    out.extend(&chars[drop_end..]);
    Ok(out)
}

/// Vector `SUBSTITUTE`. NA in / NA out.
pub fn substitute_vec(
    x: &Character,
    old: &str,
    new: &str,
    instance: Option<usize>,
) -> Character {
    let out: Vec<Option<String>> = iter_character(x)
        .map(|cell| cell.map(|s| substitute(s, old, new, instance)))
        .collect();
    Character::from_options(out)
}

/// Vector `REPLACE`. NA in / NA out; errors collapse to NA.
pub fn replace_vec(x: &Character, start: i64, length: i64, new: &str) -> Character {
    let out: Vec<Option<String>> = iter_character(x)
        .map(|cell| cell.and_then(|s| replace(s, start, length, new).ok()))
        .collect();
    Character::from_options(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::Vector;

    #[test]
    fn substitute_replaces_all_by_default() {
        assert_eq!(substitute("foo bar foo", "foo", "baz", None), "baz bar baz");
        assert_eq!(substitute("aaaa", "a", "bb", None), "bbbbbbbb");
        assert_eq!(substitute("hello", "world", "x", None), "hello");
    }

    #[test]
    fn substitute_replaces_nth_only() {
        assert_eq!(
            substitute("foo bar foo bar foo", "foo", "X", Some(2)),
            "foo bar X bar foo"
        );
        assert_eq!(
            substitute("aaaa", "a", "Z", Some(3)),
            "aaZa"
        );
        // k > number of occurrences -> unchanged
        assert_eq!(substitute("a a", "a", "X", Some(5)), "a a");
    }

    #[test]
    fn substitute_empty_old_is_noop() {
        assert_eq!(substitute("hello", "", "X", None), "hello");
        assert_eq!(substitute("hello", "", "X", Some(1)), "hello");
    }

    #[test]
    fn substitute_unicode() {
        assert_eq!(substitute("漢字漢字", "漢", "金", None), "金字金字");
        assert_eq!(substitute("漢字漢字", "漢", "金", Some(2)), "漢字金字");
        assert_eq!(substitute("a🙂b🙂c", "🙂", "_", None), "a_b_c");
    }

    #[test]
    fn substitute_instance_zero_is_noop() {
        // Implementation choice documented in module docs.
        assert_eq!(substitute("aaa", "a", "X", Some(0)), "aaa");
    }

    #[test]
    fn replace_basic() {
        // REPLACE("abcdef", 2, 3, "XY") -> "aXYef"
        assert_eq!(replace("abcdef", 2, 3, "XY").unwrap(), "aXYef");
        // Insert at start
        assert_eq!(replace("abc", 1, 0, "Z").unwrap(), "Zabc");
        // Replace nothing in middle (length=0 inserts)
        assert_eq!(replace("abc", 2, 0, "Z").unwrap(), "aZbc");
        // Replace whole string
        assert_eq!(replace("abc", 1, 3, "WXYZ").unwrap(), "WXYZ");
        // Empty new
        assert_eq!(replace("abcdef", 2, 3, "").unwrap(), "aef");
    }

    #[test]
    fn replace_clamps_past_end() {
        // Excel: start past end appends.
        assert_eq!(replace("abc", 10, 5, "Z").unwrap(), "abcZ");
        // Length past end clamps.
        assert_eq!(replace("abc", 2, 100, "Z").unwrap(), "aZ");
    }

    #[test]
    fn replace_unicode() {
        assert_eq!(replace("漢字日本", 2, 2, "X").unwrap(), "漢X本");
        assert_eq!(replace("a🙂b", 2, 1, "*").unwrap(), "a*b");
    }

    #[test]
    fn replace_bad_params() {
        assert!(replace("abc", 0, 1, "Z").is_err());
        assert!(replace("abc", -1, 1, "Z").is_err());
        assert!(replace("abc", 1, -1, "Z").is_err());
    }

    #[test]
    fn vec_variants_propagate_na() {
        let x = Character::from_options(vec![Some("foo foo".into()), None, Some("bar".into())]);
        let got = substitute_vec(&x, "foo", "X", None);
        assert_eq!(got.get(0), Some(&Some("X X".into())));
        assert!(got.is_na(1));
        assert_eq!(got.get(2), Some(&Some("bar".into())));

        let got = replace_vec(&x, 1, 2, "ZZ");
        assert_eq!(got.get(0), Some(&Some("ZZo foo".into())));
        assert!(got.is_na(1));
    }
}
