#![forbid(unsafe_code)]
//! # `unicode-normalize` — zero-dependency Unicode canonical normalization
//!
//! Turns text into a *canonical* form so that two strings that a human would
//! call "the same" compare equal byte-for-byte. The classic example: the letter
//! "é" can be stored two ways —
//!
//! - as **one** code point `U+00E9` (LATIN SMALL LETTER E WITH ACUTE), or
//! - as **two** code points `U+0065 U+0301` (a plain "e" followed by a combining
//!   acute accent).
//!
//! They look identical but are different bytes. Unicode defines *normalization
//! forms* to reconcile them:
//!
//! - **NFD** (Normalization Form D, *canonical decomposition*): expand every
//!   character to its fullest decomposed form and put combining marks in a fixed
//!   canonical order. "é" → `e` + `◌́`. This is how you strip accents: decompose,
//!   then drop the combining marks, leaving the base letters.
//! - **NFC** (Normalization Form C, *canonical composition*): decompose, then
//!   recombine into the shortest composed form. "e" + `◌́` → "é". This is the form
//!   you use to de-duplicate visually-identical strings.
//!
//! This crate implements exactly NFD and NFC plus [`char::is_combining_mark`],
//! which is all the Engram flashcard search/template code needs. It targets
//! **Unicode 17.0.0** and reproduces the third-party `unicode-normalization`
//! crate's output bit-for-bit (verified across every code point), but with
//! **no dependencies** — honouring the repository's zero-dependency policy.
//!
//! ## How it works
//!
//! Three ingredients, all in the generated [`tables`] module (produced once from
//! the Unicode Character Database and then frozen):
//!
//! 1. **Canonical Combining Class (CCC)** — a small integer per character saying
//!    how combining marks reorder relative to each other (0 = a "starter", the
//!    base characters; nonzero = a mark that attaches to a starter).
//! 2. **Canonical decomposition** — the mapping "é" → `e` `◌́`, applied
//!    recursively.
//! 3. **Canonical composition** — the reverse mapping `e` `◌́` → "é", used by NFC.
//!
//! Korean **Hangul** syllables are handled by arithmetic (the Unicode standard
//! defines them algorithmically) rather than by table, saving ~11,000 entries.

pub mod tables;

use tables::{CCC, COMPOSE, DECOMP, MARK};

pub use tables::UNICODE_VERSION;

// ---------------------------------------------------------------------------
// Hangul: the Korean syllable block is defined algorithmically by the Unicode
// standard (UAX #15, section 16), so no table is needed. A syllable code point
// S decomposes into a leading consonant (L), a vowel (V), and optionally a
// trailing consonant (T); the reverse recomposes them.
// ---------------------------------------------------------------------------
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT; // 588
const S_COUNT: u32 = L_COUNT * N_COUNT; // 11172

#[inline]
fn is_hangul_syllable(cp: u32) -> bool {
    (S_BASE..S_BASE + S_COUNT).contains(&cp)
}

/// Push the algorithmic Hangul decomposition of `cp` onto `out`.
fn decompose_hangul(cp: u32, out: &mut Vec<char>) {
    let s_index = cp - S_BASE;
    let l = L_BASE + s_index / N_COUNT;
    let v = V_BASE + (s_index % N_COUNT) / T_COUNT;
    let t = T_BASE + s_index % T_COUNT;
    // These arithmetic results are always valid scalar values by construction.
    out.push(char::from_u32(l).unwrap());
    out.push(char::from_u32(v).unwrap());
    if t != T_BASE {
        out.push(char::from_u32(t).unwrap());
    }
}

/// Try to compose two code points as Hangul (L+V or LV+T). Returns the syllable
/// code point on success.
fn compose_hangul(a: u32, b: u32) -> Option<u32> {
    // Leading consonant + vowel → LV syllable.
    if (L_BASE..L_BASE + L_COUNT).contains(&a) && (V_BASE..V_BASE + V_COUNT).contains(&b) {
        let l_index = a - L_BASE;
        let v_index = b - V_BASE;
        return Some(S_BASE + (l_index * V_COUNT + v_index) * T_COUNT);
    }
    // LV syllable + trailing consonant → LVT syllable.
    if is_hangul_syllable(a)
        && (a - S_BASE).is_multiple_of(T_COUNT)
        && (T_BASE + 1..T_BASE + T_COUNT).contains(&b)
    {
        return Some(a + (b - T_BASE));
    }
    None
}

// ---------------------------------------------------------------------------
// Table lookups — all binary searches over the sorted generated arrays.
// ---------------------------------------------------------------------------

/// The canonical combining class of a character (0 for the vast majority).
#[inline]
fn ccc(c: char) -> u8 {
    let cp = c as u32;
    match CCC.binary_search_by_key(&cp, |&(k, _)| k) {
        Ok(i) => CCC[i].1,
        Err(_) => 0,
    }
}

/// Append the canonical decomposition of `c` to `out`. The [`DECOMP`] table
/// already stores the fully-recursive form, so no recursion is needed here —
/// Hangul is the only algorithmic case.
fn push_decomposition(c: char, out: &mut Vec<char>) {
    let cp = c as u32;
    if is_hangul_syllable(cp) {
        decompose_hangul(cp, out);
        return;
    }
    match DECOMP.binary_search_by_key(&cp, |&(k, _)| k) {
        Ok(i) => out.extend_from_slice(DECOMP[i].1),
        Err(_) => out.push(c),
    }
}

/// Compose two characters into their canonical primary composite, if one exists.
fn compose(a: char, b: char) -> Option<char> {
    if let Some(cp) = compose_hangul(a as u32, b as u32) {
        return char::from_u32(cp);
    }
    let key = ((a as u64) << 21) | b as u64;
    match COMPOSE.binary_search_by_key(&key, |&(k, _)| k) {
        Ok(i) => char::from_u32(COMPOSE[i].1),
        Err(_) => None,
    }
}

/// Reorder combining marks into canonical order (UAX #15 "Canonical Ordering").
///
/// Within any run, two *adjacent* characters are swapped when the earlier one
/// has a strictly greater nonzero combining class than the later one. Repeating
/// this to a fixed point yields a stable sort of each combining-mark run by CCC,
/// which is exactly the canonical order.
fn canonical_order(chars: &mut [char]) {
    if chars.len() < 2 {
        return;
    }
    let mut swapped = true;
    while swapped {
        swapped = false;
        for i in 1..chars.len() {
            let a = ccc(chars[i - 1]);
            let b = ccc(chars[i]);
            if a != 0 && b != 0 && a > b {
                chars.swap(i - 1, i);
                swapped = true;
            }
        }
    }
}

/// Normalization Form D (canonical decomposition) of a character sequence.
fn nfd_chars(input: impl Iterator<Item = char>) -> Vec<char> {
    let mut out = Vec::new();
    for c in input {
        push_decomposition(c, &mut out);
    }
    canonical_order(&mut out);
    out
}

/// Normalization Form C (canonical composition): NFD, then recompose.
///
/// This is the standard UAX #15 composition algorithm. We walk the decomposed
/// sequence keeping the position of the last *starter* (CCC 0). A following
/// character composes onto that starter iff (a) the starter is a real starter,
/// and (b) nothing "blocks" it — i.e. the previous character was the starter
/// itself (`last_ccc == 0`) or had a strictly smaller combining class
/// (`last_ccc < ccc`). A successful composition replaces the starter in place
/// and consumes the combining character.
fn nfc_chars(input: impl Iterator<Item = char>) -> Vec<char> {
    let decomposed = nfd_chars(input);
    if decomposed.is_empty() {
        return decomposed;
    }
    let mut out: Vec<char> = Vec::with_capacity(decomposed.len());
    out.push(decomposed[0]);
    let mut last_starter = 0usize;
    let mut starter_valid = ccc(decomposed[0]) == 0;
    let mut last_ccc = ccc(decomposed[0]);

    for &c in &decomposed[1..] {
        let cc = ccc(c);
        if starter_valid && (last_ccc == 0 || last_ccc < cc) {
            if let Some(p) = compose(out[last_starter], c) {
                out[last_starter] = p;
                // `c` is consumed; the blocking class (`last_ccc`) is unchanged.
                continue;
            }
        }
        if cc == 0 {
            last_starter = out.len();
            starter_valid = true;
        }
        last_ccc = cc;
        out.push(c);
    }
    out
}

/// Character-level Unicode queries, mirroring `unicode_normalization::char`.
pub mod char {
    use super::{ccc, MARK};

    /// Whether `c` is a combining mark (General_Category = Mark: Mn, Mc, or Me).
    ///
    /// Used to strip accents after NFD: decompose, then drop everything for which
    /// this returns `true`, leaving the base letters.
    pub fn is_combining_mark(c: char) -> bool {
        let cp = c as u32;
        // MARK is a sorted list of inclusive ranges; find the range containing cp.
        MARK.binary_search_by(|&(lo, hi)| {
            if cp < lo {
                std::cmp::Ordering::Greater
            } else if cp > hi {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
    }

    /// The canonical combining class of `c` (0 for non-combining characters).
    pub fn canonical_combining_class(c: char) -> u8 {
        ccc(c)
    }
}

/// The `.nfd()` / `.nfc()` methods, mirroring the `unicode_normalization`
/// `UnicodeNormalization` trait for the two receivers Engram uses: a `&str` and
/// any `Iterator<Item = char>` (e.g. `str::chars()`).
///
/// The iterators are computed eagerly (the inputs are short search/template
/// strings), returning a `vec::IntoIter<char>` so `.collect()` and `for` loops
/// work exactly as before.
pub trait UnicodeNormalize {
    /// Iterator over the input in Normalization Form D.
    fn nfd(self) -> std::vec::IntoIter<char>;
    /// Iterator over the input in Normalization Form C.
    fn nfc(self) -> std::vec::IntoIter<char>;
}

impl UnicodeNormalize for &str {
    fn nfd(self) -> std::vec::IntoIter<char> {
        nfd_chars(self.chars()).into_iter()
    }
    fn nfc(self) -> std::vec::IntoIter<char> {
        nfc_chars(self.chars()).into_iter()
    }
}

impl UnicodeNormalize for std::str::Chars<'_> {
    fn nfd(self) -> std::vec::IntoIter<char> {
        nfd_chars(self).into_iter()
    }
    fn nfc(self) -> std::vec::IntoIter<char> {
        nfc_chars(self).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::char::{canonical_combining_class, is_combining_mark};
    use super::UnicodeNormalize;

    fn nfd(s: &str) -> String {
        s.nfd().collect()
    }
    fn nfc(s: &str) -> String {
        s.chars().nfc().collect()
    }

    #[test]
    fn version_is_17() {
        assert_eq!(super::UNICODE_VERSION, (17, 0, 0));
    }

    #[test]
    fn decomposes_precomposed_accent() {
        // "é" (U+00E9) → "e" + combining acute (U+0301)
        assert_eq!(nfd("\u{00E9}"), "e\u{0301}");
    }

    #[test]
    fn composes_back() {
        assert_eq!(nfc("e\u{0301}"), "\u{00E9}");
    }

    #[test]
    fn strips_accents_via_nfd_and_mark_filter() {
        let stripped: String = "Crème brûlée"
            .nfd()
            .filter(|c| !is_combining_mark(*c))
            .collect();
        assert_eq!(stripped, "Creme brulee");
    }

    #[test]
    fn canonical_ordering_of_multiple_marks() {
        // Combining marks in non-canonical order get reordered by CCC.
        // U+0301 (above, ccc 230) then U+0323 (below, ccc 220) must sort to
        // (0323, 0301) because 220 < 230.
        let s = "a\u{0301}\u{0323}";
        assert_eq!(nfd(s), "a\u{0323}\u{0301}");
    }

    #[test]
    fn hangul_round_trips() {
        // 각 (U+AC01) = ᄀ(U+1100) ᅡ(U+1161) ᆨ(U+11A8)
        assert_eq!(nfd("\u{AC01}"), "\u{1100}\u{1161}\u{11A8}");
        assert_eq!(nfc("\u{1100}\u{1161}\u{11A8}"), "\u{AC01}");
    }

    #[test]
    fn combining_class_basics() {
        assert_eq!(canonical_combining_class('a'), 0);
        assert_eq!(canonical_combining_class('\u{0301}'), 230);
        assert_eq!(canonical_combining_class('\u{0323}'), 220);
    }

    #[test]
    fn is_combining_mark_basics() {
        assert!(!is_combining_mark('a'));
        assert!(is_combining_mark('\u{0301}'));
        assert!(is_combining_mark('\u{11C3A}'));
    }
}
