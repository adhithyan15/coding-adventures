//! # `html_scan` — a hand-written HTML-tag stripper for search-text rendering
//!
//! Anki stores flashcard fields as HTML. When searching, that HTML is reduced to
//! plain text. One step of that reduction is stripping every tag — turning
//! `Hello <b>world</b>` into `Hello world`. This used to be done with the
//! `regex` pattern `(?is)<[^>]+>`; as part of the Engram zero-dependency program
//! we replace it with an explicit scanner so the crate needs no regular-
//! expression engine for this step.
//!
//! The pattern `<[^>]+>` is simple enough to reproduce exactly: from each `<`,
//! find the next `>`; if there is at least one character between them, that whole
//! `<…>` span is a tag and is removed. (`<>` — nothing between the brackets — is
//! *not* a tag, because `[^>]+` requires at least one character, so it is left
//! as-is.) The `(?is)` flags do not change anything here: `s` (dot-matches-
//! newline) is irrelevant because `[^>]` already includes newlines, and there is
//! no case-sensitive literal to fold. Verified byte-for-byte against the original
//! `regex` across a large random corpus.
//!
//! The *media-tag* handling (extracting a filename from `<img src=…>` etc.) is
//! deliberately **not** reimplemented here: its original regex relies on
//! alternation and quoted values that can span `>`, whose exact backtracking is
//! best reproduced by a general regex engine rather than a bespoke scanner. It
//! is handled by the forthcoming zero-dependency regex engine (Phase D) together
//! with the search-matching patterns.

/// Strip every `<…>` tag — a `<`, at least one non-`>` character, then a `>` —
/// leaving the text between tags untouched. Equivalent to the original
/// `Regex::new(r"(?is)<[^>]+>").replace_all(value, "")`.
///
/// Between tags we copy validated `&str` slices of the input, so multi-byte
/// UTF-8 passes through intact (`<` and `>` are ASCII, so every cut lands on a
/// character boundary).
pub(crate) fn strip_tags(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    let mut copied = 0; // start of the not-yet-copied region
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // `<[^>]+>`: need ≥1 non-`>` char, then a `>`.
            match bytes[i + 1..].iter().position(|&b| b == b'>') {
                Some(rel) if rel >= 1 => {
                    out.push_str(&value[copied..i]);
                    i = i + 1 + rel + 1; // resume just past the `>`
                    copied = i;
                    continue;
                }
                // `<>` (rel == 0) is not a tag; skip just this `<`.
                Some(_) => {
                    i += 1;
                    continue;
                }
                // No `>` in the remainder — and since every later position's tail
                // is a suffix of this one, no `<` at or after `i` can find a `>`
                // either. Stop scanning. (Without this, a run of `<` with no `>`
                // would re-scan the tail per `<`, an O(n²) DoS on hostile input.)
                None => break,
            }
        }
        i += 1;
    }
    out.push_str(&value[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::strip_tags;
    use regex::Regex;

    fn tag_re() -> Regex {
        Regex::new(r"(?is)<[^>]+>").unwrap()
    }

    #[test]
    fn basics() {
        assert_eq!(strip_tags("<b>hi</b> there"), "hi there");
        assert_eq!(strip_tags("a<br>b"), "ab");
        // `<>` is not a tag (`[^>]+` needs ≥1 char between the brackets).
        assert_eq!(strip_tags("a<>b"), "a<>b");
        // A `<` with no following `>` is left alone.
        assert_eq!(strip_tags("a < b"), "a < b");
        // Multi-byte content survives; only the tags are removed.
        assert_eq!(strip_tags("café<i>x</i>"), "caféx");
        // `[^>]` spans newlines (the `s` flag is a no-op here).
        assert_eq!(strip_tags("a<b\nc>d"), "ad");
    }

    // Deterministic LCG — no rng dependency.
    struct Lcg(u64);
    impl Lcg {
        fn n(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 16
        }
    }

    #[test]
    fn long_unterminated_angle_run_is_linear_and_correct() {
        // Regression guard for the O(n²) DoS: a long run of `<` with no `>` must
        // be returned unchanged (no tag) and must not re-scan the tail per `<`.
        let s: String = std::iter::repeat_n('<', 200_000).collect();
        let start = std::time::Instant::now();
        let out = strip_tags(&s);
        let dt = start.elapsed();
        assert_eq!(out, s); // no `>` anywhere ⇒ nothing is a tag
        assert!(dt.as_millis() < 200, "strip_tags not linear: {dt:?}");
        // Mixed: many `<`, one `>` near the end forms exactly one tag.
        let mut m = "<".repeat(100_000);
        m.push_str(">tail");
        assert_eq!(strip_tags(&m), "tail");
    }

    #[test]
    fn matches_regex_across_random_corpus() {
        let toks = [
            "<",
            ">",
            "<b>",
            "</b>",
            "<br>",
            "<>",
            "a",
            "b",
            "café",
            " ",
            "\n",
            "\t",
            "<i",
            "x>",
            "hello",
            "<img src=",
            "\"y\"",
            "z",
            "<<",
            ">>",
            "</",
            "/>",
        ];
        let re = tag_re();
        let mut rng = Lcg(0x5CA1AB1E);
        for _ in 0..300_000 {
            let len = 1 + (rng.n() % 14) as usize;
            let s: String = (0..len)
                .map(|_| toks[(rng.n() as usize) % toks.len()])
                .collect();
            let mine = strip_tags(&s);
            let re_out = re.replace_all(&s, "").into_owned();
            assert_eq!(mine, re_out, "strip mismatch on {s:?}");
        }
    }
}
