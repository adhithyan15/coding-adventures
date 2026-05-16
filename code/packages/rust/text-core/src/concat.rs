//! CONCAT / CONCATENATE / TEXTJOIN — string composition.
//!
//! - `concatenate(strs)` — joins all strings with empty separator. This is
//!   the venerable Excel `CONCATENATE`.
//! - `concat(strs)` — modern Excel `CONCAT`. Same as `CONCATENATE` for our
//!   purposes (we don't model spreadsheet ranges here; the caller flattens).
//! - `textjoin(delim, ignore_empty, parts)` — joins with `delim`. If
//!   `ignore_empty` is true, empty strings are skipped (no extra delimiter).
//!
//! NA handling: at this layer, all inputs are `&str` slices. NA is handled by
//! the bridge: a NA element should be passed as either an empty string (when
//! the caller wants "blank" semantics) or surfaced upstream as `#N/A`. The
//! `*_options` variants below take `Option<&str>` slices to make the choice
//! explicit:
//!
//! - `concat_options(parts)` — `None` becomes empty string (Excel CONCAT
//!   behaviour for blank cells).
//! - `textjoin_options(delim, ignore_empty, parts)` — `None` is treated like
//!   an empty string when `ignore_empty` is true.

use crate::iter_character;
use r_vector::Character;

/// `CONCATENATE(s1, s2, ...)`.
pub fn concatenate(parts: &[&str]) -> String {
    let mut total = 0;
    for p in parts {
        total += p.len();
    }
    let mut out = String::with_capacity(total);
    for p in parts {
        out.push_str(p);
    }
    out
}

/// `CONCAT(s1, s2, ...)` — same as `CONCATENATE` at this layer.
pub fn concat(parts: &[&str]) -> String {
    concatenate(parts)
}

/// `CONCAT` over `Option<&str>` parts. `None` -> empty.
pub fn concat_options(parts: &[Option<&str>]) -> String {
    let mut out = String::new();
    for p in parts {
        if let Some(s) = p {
            out.push_str(s);
        }
    }
    out
}

/// `TEXTJOIN(delim, ignore_empty, parts)`.
pub fn textjoin(delim: &str, ignore_empty: bool, parts: &[&str]) -> String {
    let mut first = true;
    let mut out = String::new();
    for p in parts {
        if ignore_empty && p.is_empty() {
            continue;
        }
        if first {
            first = false;
        } else {
            out.push_str(delim);
        }
        out.push_str(p);
    }
    out
}

/// `TEXTJOIN` over `Option<&str>` parts. `None` is treated as empty: skipped
/// when `ignore_empty` is true, included as nothing (causing back-to-back
/// delimiters) when false.
pub fn textjoin_options(delim: &str, ignore_empty: bool, parts: &[Option<&str>]) -> String {
    let mut first = true;
    let mut out = String::new();
    for p in parts {
        let s = p.unwrap_or("");
        if ignore_empty && s.is_empty() {
            continue;
        }
        if first {
            first = false;
        } else {
            out.push_str(delim);
        }
        out.push_str(s);
    }
    out
}

/// Concatenate a `Character` vector with the given delimiter.
///
/// `None` (NA) elements are skipped iff `ignore_empty` is true; otherwise
/// they contribute an empty slot. Convenience wrapper over
/// `textjoin_options`.
pub fn textjoin_vec(delim: &str, ignore_empty: bool, x: &Character) -> String {
    let parts: Vec<Option<&str>> = iter_character(x).collect();
    textjoin_options(delim, ignore_empty, &parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concatenate_basic() {
        assert_eq!(concatenate(&["a", "b", "c"]), "abc");
        assert_eq!(concatenate(&[]), "");
        assert_eq!(concatenate(&[""]), "");
        assert_eq!(concatenate(&["hello", " ", "world"]), "hello world");
    }

    #[test]
    fn concat_is_concatenate() {
        assert_eq!(concat(&["x", "y"]), "xy");
    }

    #[test]
    fn concat_options_treats_none_as_empty() {
        assert_eq!(concat_options(&[Some("a"), None, Some("b")]), "ab");
        assert_eq!(concat_options(&[]), "");
    }

    #[test]
    fn textjoin_basic() {
        assert_eq!(textjoin(", ", true, &["a", "b", "c"]), "a, b, c");
        assert_eq!(textjoin(", ", true, &["a", "", "c"]), "a, c");
        assert_eq!(textjoin(", ", false, &["a", "", "c"]), "a, , c");
        assert_eq!(textjoin("-", true, &[]), "");
    }

    #[test]
    fn textjoin_empty_delim() {
        assert_eq!(textjoin("", true, &["a", "b", "c"]), "abc");
    }

    #[test]
    fn textjoin_options_skips_none_when_ignoring() {
        let parts = vec![Some("a"), None, Some("b")];
        assert_eq!(textjoin_options(",", true, &parts), "a,b");
        assert_eq!(textjoin_options(",", false, &parts), "a,,b");
    }

    #[test]
    fn textjoin_vec_walks_character() {
        let x = Character::from_options(vec![
            Some("a".into()),
            None,
            Some("".into()),
            Some("c".into()),
        ]);
        assert_eq!(textjoin_vec(",", true, &x), "a,c");
        assert_eq!(textjoin_vec(",", false, &x), "a,,,c");
    }

    #[test]
    fn concat_unicode() {
        assert_eq!(concatenate(&["漢", "字"]), "漢字");
    }
}
