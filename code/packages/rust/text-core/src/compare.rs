//! EXACT — case-sensitive string equality.
//!
//! `EXACT(a, b)` returns `true` iff the two strings are byte-for-byte equal.
//! This is the standard Excel semantics: `EXACT("HELLO", "hello") == false`.

use r_vector::{Character, Vector};

/// `EXACT(a, b)` — case-sensitive equality.
pub fn exact(a: &str, b: &str) -> bool {
    a == b
}

/// Vector `EXACT`. Pairs `Character` vectors element-wise. Mismatched lengths
/// pad with NA results. NA-vs-anything yields `None` (NA boolean).
pub fn exact_vec(a: &Character, b: &Character) -> Vec<Option<bool>> {
    let n = a.len().max(b.len());
    let mut out: Vec<Option<bool>> = Vec::with_capacity(n);
    for i in 0..n {
        let ai = match a.get(i) {
            Some(Some(s)) => Some(s.as_str()),
            _ => None,
        };
        let bi = match b.get(i) {
            Some(Some(s)) => Some(s.as_str()),
            _ => None,
        };
        match (ai, bi) {
            (Some(x), Some(y)) => out.push(Some(exact(x, y))),
            _ => out.push(None),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_basic() {
        assert!(exact("hello", "hello"));
        assert!(!exact("hello", "Hello"));
        assert!(!exact("hello", "hello "));
        assert!(exact("", ""));
    }

    #[test]
    fn exact_unicode() {
        assert!(exact("漢字", "漢字"));
        assert!(!exact("漢", "字"));
        // Different code-points that *look* the same are not equal.
        assert!(!exact("e\u{0301}", "é")); // combining accent vs composed
    }

    #[test]
    fn exact_vec_pairs_elementwise() {
        let a = Character::from_options(vec![
            Some("a".into()),
            Some("B".into()),
            None,
            Some("d".into()),
        ]);
        let b = Character::from_options(vec![
            Some("a".into()),
            Some("b".into()),
            Some("c".into()),
            None,
        ]);
        assert_eq!(
            exact_vec(&a, &b),
            vec![Some(true), Some(false), None, None]
        );
    }

    #[test]
    fn exact_vec_uneven_lengths() {
        let a = Character::from_strings(["a", "b"]);
        let b = Character::from_strings(["a"]);
        assert_eq!(exact_vec(&a, &b), vec![Some(true), None]);
    }
}
