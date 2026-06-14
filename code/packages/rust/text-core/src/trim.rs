//! TRIM / CLEAN — whitespace and control-character normalisation.
//!
//! ## TRIM — Excel semantics
//!
//! `TRIM` in Excel is *not* the same as Rust's `str::trim`. It:
//!
//! 1. Removes leading and trailing ASCII space (U+0020) characters.
//! 2. Collapses each internal run of multiple spaces into a single space.
//!
//! It only operates on `' '` (U+0020). Tabs and newlines are left alone.
//! (Modern Excel also leaves non-breaking spaces U+00A0 alone; we match
//! that.)
//!
//! ```text
//! trim("  hello   world  ") == "hello world"
//! trim("a   b   c")         == "a b c"
//! trim("\t a \t")           == "\t a \t"      (tabs preserved)
//! ```
//!
//! ## CLEAN
//!
//! `CLEAN` removes all characters whose code-point is in `0..=31` (the C0
//! control range, including newline / tab / null). Higher-range control
//! characters (DEL, C1) are left alone to match Excel.

use crate::iter_character;
use r_vector::Character;

/// `TRIM(text)` — collapse runs of spaces; remove leading/trailing spaces.
pub fn trim(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_run_of_space = false;
    let mut emitted_any = false;
    for c in s.chars() {
        if c == ' ' {
            // Defer the space; we only emit it when we know there's
            // non-space content to follow.
            in_run_of_space = true;
        } else {
            if in_run_of_space && emitted_any {
                out.push(' ');
            }
            out.push(c);
            emitted_any = true;
            in_run_of_space = false;
        }
    }
    out
}

/// `CLEAN(text)` — drop characters with code points 0..=31.
pub fn clean(s: &str) -> String {
    s.chars().filter(|c| (*c as u32) > 31).collect()
}

/// Vector `TRIM`. NA in / NA out.
pub fn trim_vec(x: &Character) -> Character {
    let out: Vec<Option<String>> = iter_character(x).map(|cell| cell.map(trim)).collect();
    Character::from_options(out)
}

/// Vector `CLEAN`. NA in / NA out.
pub fn clean_vec(x: &Character) -> Character {
    let out: Vec<Option<String>> = iter_character(x).map(|cell| cell.map(clean)).collect();
    Character::from_options(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::Vector;

    #[test]
    fn trim_collapses_internal_runs() {
        assert_eq!(trim("  hello   world  "), "hello world");
        assert_eq!(trim("a   b   c"), "a b c");
        assert_eq!(trim("   "), "");
        assert_eq!(trim(""), "");
        assert_eq!(trim("nospace"), "nospace");
    }

    #[test]
    fn trim_preserves_non_space_whitespace() {
        // Tabs and newlines are preserved.
        assert_eq!(trim("\t a \t"), "\t a \t");
        assert_eq!(trim("a\nb"), "a\nb");
        // Non-breaking space is left alone.
        assert_eq!(trim("a\u{00A0}\u{00A0}b"), "a\u{00A0}\u{00A0}b");
    }

    #[test]
    fn trim_unicode_passthrough() {
        assert_eq!(trim("  漢  字  "), "漢 字");
        assert_eq!(trim("🙂   🎉"), "🙂 🎉");
    }

    #[test]
    fn clean_drops_c0_controls() {
        assert_eq!(clean("hello\nworld"), "helloworld");
        assert_eq!(clean("a\tb\rc"), "abc");
        assert_eq!(clean("\0\x01abc\x1f"), "abc");
        assert_eq!(clean(""), "");
    }

    #[test]
    fn clean_keeps_higher_chars() {
        // Space (U+0020) is kept; we only strip 0..=31.
        assert_eq!(clean(" a b "), " a b ");
        // DEL (U+007F) is intentionally left alone (Excel parity).
        assert_eq!(clean("a\x7fb"), "a\x7fb");
        assert_eq!(clean("漢字"), "漢字");
    }

    #[test]
    fn vec_variants_propagate_na() {
        let x = Character::from_options(vec![Some("  a  b  ".into()), None]);
        let t = trim_vec(&x);
        assert_eq!(t.get(0), Some(&Some("a b".into())));
        assert!(t.is_na(1));

        let c = clean_vec(&Character::from_options(vec![Some("x\ty".into()), None]));
        assert_eq!(c.get(0), Some(&Some("xy".into())));
        assert!(c.is_na(1));
    }
}
