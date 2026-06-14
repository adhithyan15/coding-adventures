//! LEN / LENB — string length.
//!
//! - `LEN` returns the number of Unicode scalar values (Rust `char`s).
//! - `LENB` returns the number of UTF-8 bytes.
//!
//! In Excel on a Latin-1 build, LEN and LENB are equivalent. On a DBCS build,
//! `LENB("é")` is 1 and `LENB("漢")` is 2. We model the latter behaviour using
//! UTF-8 byte counts, which is the modern equivalent and matches the open
//! spreadsheet (e.g. LibreOffice) convention.
//!
//! ```text
//! len("hello")  == 5
//! len("héllo")  == 5   (5 scalar values)
//! lenb("héllo") == 6   (the 'é' is 2 UTF-8 bytes)
//! len("漢字")   == 2
//! lenb("漢字")  == 6   (each CJK ideograph is 3 UTF-8 bytes)
//! ```

use crate::iter_character;
use r_vector::Character;

/// `LEN(text)` — number of Unicode scalar values.
pub fn len(s: &str) -> usize {
    s.chars().count()
}

/// `LENB(text)` — number of bytes in the UTF-8 encoding.
pub fn lenb(s: &str) -> usize {
    s.len()
}

/// Vector `LEN` over a `Character`. NA propagates: an NA element produces an
/// NA-coded length value here represented as `None`.
///
/// The output is a parallel `Vec<Option<usize>>` rather than a `Double`
/// because keeping the integer flavour avoids forcing a numeric crate
/// dependency on the caller side. The bridge layer can convert this to
/// whichever spreadsheet-numeric vector its frontend wants.
pub fn len_vec(x: &Character) -> Vec<Option<usize>> {
    iter_character(x).map(|cell| cell.map(len)).collect()
}

/// Vector `LENB`. NA propagates.
pub fn lenb_vec(x: &Character) -> Vec<Option<usize>> {
    iter_character(x).map(|cell| cell.map(lenb)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_counts_ascii() {
        assert_eq!(len("hello"), 5);
        assert_eq!(len(""), 0);
        assert_eq!(len("a"), 1);
    }

    #[test]
    fn len_counts_scalars_not_bytes() {
        assert_eq!(len("héllo"), 5);
        assert_eq!(len("漢字"), 2);
        // emoji is one scalar value
        assert_eq!(len("🙂"), 1);
    }

    #[test]
    fn lenb_counts_bytes() {
        assert_eq!(lenb("hello"), 5);
        assert_eq!(lenb("héllo"), 6);
        assert_eq!(lenb("漢字"), 6);
        assert_eq!(lenb("🙂"), 4);
    }

    #[test]
    fn len_vec_propagates_na() {
        let x = Character::from_options(vec![
            Some("hi".into()),
            None,
            Some("".into()),
            Some("漢".into()),
        ]);
        assert_eq!(len_vec(&x), vec![Some(2), None, Some(0), Some(1)]);
    }

    #[test]
    fn lenb_vec_propagates_na() {
        let x = Character::from_options(vec![Some("a".into()), None, Some("é".into())]);
        assert_eq!(lenb_vec(&x), vec![Some(1), None, Some(2)]);
    }
}
