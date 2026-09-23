//! Text normalisation shared by tags, journal names, and search.
//!
//! Three small jobs, kept in one place so every caller agrees on them:
//!
//! 1. **Tidy** a user-typed label: trim the ends and collapse every run of inner
//!    whitespace to one space, so `"  road   trip "` and `"road trip"` are the
//!    same name.
//! 2. **Fold** a string for case-insensitive comparison: `"Travel"` and
//!    `"TRAVEL"` fold to the same key.
//! 3. **Fold with an offset map**, for search. This is the subtle one.
//!
//! ## Why search needs an offset map
//!
//! The obvious way to search case-insensitively is: lowercase the body, find the
//! query in it, and slice the *original* body at the offset you found. That is
//! wrong, because lowercasing can change a string's **byte length**:
//!
//! ```text
//!   original:  "İstanbul"   'İ' is 2 bytes (U+0130)
//!   folded:    "i̇stanbul"   → 'i' + U+0307 COMBINING DOT ABOVE = 3 bytes
//! ```
//!
//! Every offset after the `İ` is now off by one, and slicing the original at a
//! folded offset either returns the wrong text or panics on a non-character
//! boundary. So [`fold_with_map`] returns, alongside the folded string, the
//! original byte offset that each folded byte came from. A match at folded offset
//! `k` is at original offset `map[k]` — always a character boundary.

/// Trim and collapse inner whitespace runs to a single ASCII space.
pub(crate) fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Case-fold for comparison (Unicode lowercase, character by character).
pub(crate) fn fold(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// Case-fold `s`, and record for each byte of the folded string the byte offset in
/// `s` of the character it came from. `map.len() == folded.len() + 1`; the final
/// entry is `s.len()`, so a match ending at the end of the folded string also maps.
pub(crate) fn fold_with_map(s: &str) -> (String, Vec<usize>) {
    let mut folded = String::with_capacity(s.len());
    let mut map = Vec::with_capacity(s.len() + 1);
    for (orig, ch) in s.char_indices() {
        for lower in ch.to_lowercase() {
            let before = folded.len();
            folded.push(lower);
            // Every byte of this folded char points back at the source char.
            map.extend(core::iter::repeat_n(orig, folded.len() - before));
        }
    }
    map.push(s.len());
    (folded, map)
}

/// True if `s` contains a control character (newline, tab, NUL, …). Labels are
/// single-line display strings; a control character in one is either a paste
/// accident or an attempt to break a host's rendering.
pub(crate) fn has_control(s: &str) -> bool {
    s.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_trims_and_collapses() {
        assert_eq!(tidy("  road \t  trip \n"), "road trip");
        assert_eq!(tidy(""), "");
        assert_eq!(tidy("   "), "");
    }

    #[test]
    fn fold_is_case_insensitive_beyond_ascii() {
        assert_eq!(fold("Travel"), fold("TRAVEL"));
        assert_eq!(fold("ÉTÉ"), "été");
    }

    #[test]
    fn fold_map_points_every_folded_byte_at_a_source_char_boundary() {
        let s = "İstanbul café";
        let (folded, map) = fold_with_map(s);
        assert_eq!(map.len(), folded.len() + 1);
        for &o in &map {
            assert!(s.is_char_boundary(o));
        }
        // 'İ' grew from 2 to 3 bytes, so a naive offset would be wrong by one.
        let at = folded.find("stanbul").unwrap();
        assert_eq!(&s[map[at]..], "stanbul café");
        assert_eq!(*map.last().unwrap(), s.len());
    }

    #[test]
    fn control_characters_are_detected() {
        assert!(has_control("a\nb"));
        assert!(has_control("\u{0}"));
        assert!(!has_control("plain label"));
    }
}
