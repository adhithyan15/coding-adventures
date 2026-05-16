//! UPPER / LOWER / PROPER — case conversion.
//!
//! - `UPPER` and `LOWER` use Rust's Unicode-aware default case folding.
//!   Note that some characters *grow* (e.g. `"ß".to_uppercase() == "SS"`), so
//!   the output may have a different `char` count than the input.
//! - `PROPER` capitalises the first letter of each *word*. A word is defined
//!   as a maximal run of `char::is_alphabetic` characters. All other
//!   characters reset the word-start flag. Apostrophes count as
//!   non-alphabetic (matches Excel: `proper("o'brien") == "O'Brien"`).
//!
//! ```text
//! upper("hello") == "HELLO"
//! lower("HELLO") == "hello"
//! upper("straße") == "STRASSE"
//! proper("the quick brown fox") == "The Quick Brown Fox"
//! proper("o'brien")             == "O'Brien"
//! proper("html5 is cool")       == "Html5 Is Cool"
//! ```

use crate::iter_character;
use r_vector::Character;

/// `UPPER(text)` — Unicode-aware uppercase.
pub fn upper(s: &str) -> String {
    s.to_uppercase()
}

/// `LOWER(text)` — Unicode-aware lowercase.
pub fn lower(s: &str) -> String {
    s.to_lowercase()
}

/// `PROPER(text)` — capitalise the first letter of each word; everything else
/// lowercase. A "word" is a run of `is_alphabetic` characters.
pub fn proper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // `at_word_start` tracks whether the next alphabetic char should be made
    // uppercase. It is true at the very start and after every non-alphabetic
    // character.
    let mut at_word_start = true;
    for c in s.chars() {
        if c.is_alphabetic() {
            if at_word_start {
                // Push the uppercase form (may be multi-char).
                for u in c.to_uppercase() {
                    out.push(u);
                }
            } else {
                for l in c.to_lowercase() {
                    out.push(l);
                }
            }
            at_word_start = false;
        } else {
            out.push(c);
            at_word_start = true;
        }
    }
    out
}

/// Vector `UPPER`. NA in / NA out.
pub fn upper_vec(x: &Character) -> Character {
    let out: Vec<Option<String>> = iter_character(x).map(|cell| cell.map(upper)).collect();
    Character::from_options(out)
}

/// Vector `LOWER`. NA in / NA out.
pub fn lower_vec(x: &Character) -> Character {
    let out: Vec<Option<String>> = iter_character(x).map(|cell| cell.map(lower)).collect();
    Character::from_options(out)
}

/// Vector `PROPER`. NA in / NA out.
pub fn proper_vec(x: &Character) -> Character {
    let out: Vec<Option<String>> = iter_character(x).map(|cell| cell.map(proper)).collect();
    Character::from_options(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::Vector;

    #[test]
    fn upper_basic() {
        assert_eq!(upper("hello"), "HELLO");
        assert_eq!(upper(""), "");
        assert_eq!(upper("HeLLo"), "HELLO");
    }

    #[test]
    fn upper_unicode_growth() {
        assert_eq!(upper("straße"), "STRASSE");
        assert_eq!(upper("ﬃ"), "FFI"); // U+FB03 ligature
    }

    #[test]
    fn lower_basic() {
        assert_eq!(lower("HELLO"), "hello");
        assert_eq!(lower("Mixed Case"), "mixed case");
    }

    #[test]
    fn lower_unicode() {
        assert_eq!(lower("ÉTÉ"), "été");
    }

    #[test]
    fn proper_basic() {
        assert_eq!(proper("hello world"), "Hello World");
        assert_eq!(proper("HELLO WORLD"), "Hello World");
        assert_eq!(proper(""), "");
    }

    #[test]
    fn proper_apostrophes_count_as_breaks() {
        // Excel parity: each letter cluster gets its own capitalisation.
        assert_eq!(proper("o'brien"), "O'Brien");
    }

    #[test]
    fn proper_digits_and_punct_break_words() {
        assert_eq!(proper("html5 is cool"), "Html5 Is Cool");
        assert_eq!(proper("anne-marie"), "Anne-Marie");
    }

    #[test]
    fn proper_unicode_words() {
        assert_eq!(proper("école normale"), "École Normale");
    }

    #[test]
    fn vec_variants_propagate_na() {
        let x = Character::from_options(vec![
            Some("hello".into()),
            None,
            Some("WORLD".into()),
            Some("a b c".into()),
        ]);
        let up = upper_vec(&x);
        assert_eq!(up.get(0), Some(&Some("HELLO".into())));
        assert!(up.is_na(1));
        assert_eq!(up.get(2), Some(&Some("WORLD".into())));

        let lo = lower_vec(&x);
        assert_eq!(lo.get(2), Some(&Some("world".into())));
        assert!(lo.is_na(1));

        let pr = proper_vec(&x);
        assert_eq!(pr.get(3), Some(&Some("A B C".into())));
        assert!(pr.is_na(1));
    }
}
