#![forbid(unsafe_code)]
//! # `regex-engine` — a small, zero-dependency, linear-time regex engine
//!
//! A from-scratch regular-expression engine covering the subset of syntax the
//! Engram flashcard search needs, built so the Engram stack needs no third-party
//! regex crate (honouring the repository's zero-dependency policy).
//!
//! ## Why a Thompson NFA / Pike VM (and not backtracking)
//!
//! The obvious way to match a regex is recursive backtracking, but that can take
//! **exponential** time on adversarial patterns (e.g. `(a*)*b` against a long
//! run of `a`s) — a denial-of-service waiting to happen, and Engram runs
//! *user-supplied* `re:` patterns. Instead this engine compiles the pattern to a
//! tiny bytecode and runs a **Pike VM**: it advances every possible match
//! through the input in lockstep, so matching is always **O(pattern × input)**,
//! never exponential. Capture groups are tracked per thread, and ambiguity is
//! resolved *leftmost-first* with greedy quantifiers preferring to match more —
//! the same semantics the `regex` crate uses for these patterns.
//!
//! ## Supported syntax
//!
//! Literals; `.`; the escapes `\d \D \w \W \s \S` and escaped metacharacters;
//! the Unicode property classes `\p{Alphabetic}`, `\p{Mark}`, `\p{Nd}` (and
//! `\P{…}`); character classes `[...]`/`[^...]` with ranges; groups `(...)` and
//! `(?:...)`; alternation `|`; the quantifiers `* + ?` and `{m}`/`{m,}`/`{m,n}`,
//! greedy or lazy (`*?` …); the anchors `^ $`; word boundaries `\b \B`; and the
//! leading inline flags `(?i)` / `(?s)` / `(?u)`.
//!
//! Character classes and `\b` are **Unicode-aware by default** (matching the
//! `regex` crate), backed by generated tables in [`unicode_tables`]; `(?-u)`
//! selects the ASCII sets. `(?i)` uses Unicode simple case folding. Boolean
//! matching ([`Regex::is_match`]), the overall match *extent* ([`Regex::find`]),
//! capture groups ([`Regex::captures`]), the match iterators
//! ([`Regex::find_iter`]/[`Regex::captures_iter`]), and [`Regex::replace_all`] are
//! all implemented — the full surface Engram's search and media-tag replacement
//! need.

mod ast;
mod casefold;
mod program;
mod unicode_tables;

use ast::Flags;
use program::{Input, Program, RawCaptures};
use std::borrow::Cow;

/// An error building or running a regular expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

/// The maximum number of compiled instructions. A pattern that would compile to
/// more (e.g. `a{0,10000000}`) is rejected, bounding memory and match time —
/// mirroring the `regex` crate's compile-size limit.
const MAX_PROGRAM_INSTS: usize = 200_000;

/// A compiled regular expression.
#[derive(Debug, Clone)]
pub struct Regex {
    program: Program,
}

impl Regex {
    /// Compile `pattern`. Returns [`Error`] on invalid syntax or an over-large
    /// program.
    pub fn new(pattern: &str) -> Result<Regex, Error> {
        RegexBuilder::new(pattern).build()
    }

    /// Whether the pattern matches anywhere in `text`.
    ///
    /// This is the operation Engram's search needs (`re:`, whole-word, and glob
    /// filters are all boolean matches). It is cross-verified byte-for-byte
    /// against the `regex` crate (in `(?-u)` mode) across a large random corpus.
    /// For the boundaries of a match, use [`find`](Self::find).
    pub fn is_match(&self, text: &str) -> bool {
        let input = Input::new(text);
        self.program.is_match_from(&input, 0)
    }

    /// The leftmost match in `text`, or `None` if the pattern does not match.
    ///
    /// Returns the overall match *extent* (byte range + substring), resolving
    /// ambiguity **leftmost-first** with greedy quantifiers preferring to match
    /// more — the textbook Pike-VM semantics. It always returns a *valid* match at
    /// the *leftmost* possible start (cross-checked as a property against the
    /// `regex` crate's anchored matcher over a large random corpus). For the
    /// overwhelming majority of patterns it reports the same byte boundaries as the
    /// `regex` crate, including nullable loops such as `(a?)*` (which matches the
    /// whole run of `a`s, not the empty prefix — see the star compilation in
    /// `program.rs`).
    ///
    /// The reported *extent* can differ from the `regex` crate on some adversarial
    /// patterns — chiefly around **lazy** quantifiers and **overlapping greedy
    /// alternation** (e.g. `.+c+|.+.+`) — because the `regex` crate's specific NFA
    /// thread-priority resolves those ambiguities differently from textbook
    /// leftmost-first (in some cases its unanchored `find` even skips the leftmost
    /// match its own anchored matcher accepts). [`is_match`](Self::is_match) stays
    /// exact regardless. Engram's only extent consumer — the media-tag regex — is
    /// greedy with disjoint alternation, so it is unaffected. Capture groups and
    /// `replace_all` build on this.
    pub fn find<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        let input = Input::new(text);
        self.program
            .find_from(&input, 0)
            .map(|(start, end)| Match { text, start, end })
    }

    /// The leftmost match in `text` with its capture groups, or `None`.
    ///
    /// [`Captures::get(0)`](Captures::get) is the overall match (same extent as
    /// [`find`](Self::find)); `get(i)` for `i ≥ 1` is the `i`-th capturing group
    /// `(…)`, or `None` if that group did not participate in the match. Capture
    /// boundaries use the `regex` crate's leftmost-first Pike-VM semantics, so on
    /// the patterns Engram builds (`re:`, whole-word, and the media-tag regex — no
    /// nested lazy-nullable groups) they agree with `regex` byte-for-byte.
    ///
    /// A pattern with more than 1000 capturing groups is rejected at
    /// [`RegexBuilder::build`] time (a DoS guard on per-thread slot state).
    pub fn captures<'t>(&self, text: &'t str) -> Option<Captures<'t>> {
        let input = Input::new(text);
        self.program
            .captures_from(&input, 0)
            .map(|slots| Captures { text, slots })
    }

    /// Iterate the leftmost, **non-overlapping** matches in `text`, yielding each
    /// match's overall extent. Same iteration semantics as the `regex` crate: after
    /// a match the search resumes at its end, and an *empty* match immediately
    /// adjacent to the previous match's end is skipped (so `a*` on `"aba"` yields
    /// the two `a`s and the empty gaps, not a doubled empty match).
    pub fn find_iter<'r, 't>(&'r self, text: &'t str) -> FindIter<'r, 't> {
        FindIter {
            it: Searcher::new(&self.program, text),
            text,
        }
    }

    /// Iterate the leftmost, non-overlapping matches in `text` with their capture
    /// groups. Same non-overlapping semantics as [`find_iter`](Self::find_iter).
    pub fn captures_iter<'r, 't>(&'r self, text: &'t str) -> CapturesIter<'r, 't> {
        CapturesIter {
            it: Searcher::new(&self.program, text),
            text,
        }
    }

    /// Replace every non-overlapping match in `text` using `rep`, returning the
    /// result (borrowed unchanged when there is no match). `rep` may be a closure
    /// `FnMut(&Captures) -> String` (given each match's captures, produce its
    /// replacement) or a replacement **string** with `$N`/`${N}` numbered-group
    /// references and `$$` for a literal `$` — see [`Replacer`].
    pub fn replace_all<'t, R: Replacer>(&self, text: &'t str, mut rep: R) -> Cow<'t, str> {
        let mut searcher = Searcher::new(&self.program, text);
        let mut out: Option<String> = None;
        let mut last_byte = 0usize;
        while let Some(raw) = searcher.next_raw() {
            // Slots 0/1 are always set on a match.
            let start = raw.byte_slots[0].expect("overall start");
            let end = raw.byte_slots[1].expect("overall end");
            let dst = out.get_or_insert_with(|| String::with_capacity(text.len()));
            dst.push_str(&text[last_byte..start]);
            let caps = Captures {
                text,
                slots: raw.byte_slots,
            };
            rep.replace_append(&caps, dst);
            last_byte = end;
        }
        match out {
            Some(mut dst) => {
                dst.push_str(&text[last_byte..]);
                Cow::Owned(dst)
            }
            None => Cow::Borrowed(text),
        }
    }
}

/// Drives leftmost non-overlapping iteration over a compiled program, mirroring
/// the `regex` crate's match-iterator semantics (resume at the previous match's
/// end; skip an empty match sitting exactly at that end). Works in char positions
/// for control and reports byte offsets.
struct Searcher<'r> {
    program: &'r Program,
    input: Input,
    from: usize,             // next char index to search from
    last_end: Option<usize>, // char index of the previous match's end
}

impl<'r> Searcher<'r> {
    fn new(program: &'r Program, text: &str) -> Self {
        Searcher {
            program,
            input: Input::new(text),
            from: 0,
            last_end: None,
        }
    }

    fn next_raw(&mut self) -> Option<RawCaptures> {
        loop {
            let raw = self.program.captures_at(&self.input, self.from)?;
            if raw.start_char == raw.end_char {
                // Empty match: always step forward one char so we make progress…
                self.from = raw.end_char + 1;
                // …and skip it if it sits exactly where the previous match ended.
                if Some(raw.end_char) == self.last_end {
                    continue;
                }
            } else {
                self.from = raw.end_char;
            }
            self.last_end = Some(raw.end_char);
            return Some(raw);
        }
    }
}

/// Iterator over overall match extents; see [`Regex::find_iter`].
pub struct FindIter<'r, 't> {
    it: Searcher<'r>,
    text: &'t str,
}

impl<'t> Iterator for FindIter<'_, 't> {
    type Item = Match<'t>;
    fn next(&mut self) -> Option<Match<'t>> {
        let raw = self.it.next_raw()?;
        Some(Match {
            text: self.text,
            start: raw.byte_slots[0].expect("overall start"),
            end: raw.byte_slots[1].expect("overall end"),
        })
    }
}

/// Iterator over per-match capture groups; see [`Regex::captures_iter`].
pub struct CapturesIter<'r, 't> {
    it: Searcher<'r>,
    text: &'t str,
}

impl<'t> Iterator for CapturesIter<'_, 't> {
    type Item = Captures<'t>;
    fn next(&mut self) -> Option<Captures<'t>> {
        let raw = self.it.next_raw()?;
        Some(Captures {
            text: self.text,
            slots: raw.byte_slots,
        })
    }
}

/// A replacement source for [`Regex::replace_all`]. Implemented for closures
/// `FnMut(&Captures) -> String` and for replacement strings (`$N`/`${N}` numbered
/// groups, `$$` → `$`).
pub trait Replacer {
    /// Append the replacement for `caps` to `dst`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String);
}

impl<F> Replacer for F
where
    F: FnMut(&Captures<'_>) -> String,
{
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        dst.push_str(&self(caps));
    }
}

impl Replacer for &str {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand_replacement(self, caps, dst);
    }
}

impl Replacer for String {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand_replacement(self, caps, dst);
    }
}

/// Expand a replacement string's `$`-references against `caps`, appending to `dst`:
/// `$N` / `${N}` insert group `N`'s text (empty if it did not participate), `$$`
/// inserts a literal `$`, and any other `$` is kept verbatim. (Named groups are
/// not supported — this engine has no named-group syntax.)
fn expand_replacement(rep: &str, caps: &Captures<'_>, dst: &mut String) {
    let bytes = rep.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            // Copy the run up to the next `$` in one push (valid UTF-8 boundary:
            // `$` is ASCII, so slicing at these indices is safe).
            let start = i;
            while i < bytes.len() && bytes[i] != b'$' {
                i += 1;
            }
            dst.push_str(&rep[start..i]);
            continue;
        }
        // At a `$`.
        i += 1;
        if i >= bytes.len() {
            dst.push('$');
            break;
        }
        match bytes[i] {
            b'$' => {
                dst.push('$');
                i += 1;
            }
            b'{' => {
                // `${N}` — read digits until `}`.
                let num_start = i + 1;
                let mut j = num_start;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'}' && j > num_start {
                    let n: usize = rep[num_start..j].parse().unwrap_or(usize::MAX);
                    if let Some(m) = caps.get(n) {
                        dst.push_str(m.as_str());
                    }
                    i = j + 1;
                } else {
                    // Malformed `${…}` — emit the `$` literally and continue.
                    dst.push('$');
                }
            }
            b'0'..=b'9' => {
                let num_start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let n: usize = rep[num_start..i].parse().unwrap_or(usize::MAX);
                if let Some(m) = caps.get(n) {
                    dst.push_str(m.as_str());
                }
            }
            _ => dst.push('$'), // a lone `$` before a non-special char
        }
    }
}

/// The capture groups of a single match. Slot pairs index into the searched text
/// by **byte** offset: group `i` spans `slots[2i]..slots[2i+1]` (each `Option`
/// because a group may not participate). Group `0` is the overall match.
#[derive(Debug, Clone)]
pub struct Captures<'t> {
    text: &'t str,
    slots: Vec<Option<usize>>,
}

impl<'t> Captures<'t> {
    /// The `i`-th capture group's match (`0` = the overall match), or `None` if
    /// the group index is out of range or the group did not participate.
    pub fn get(&self, i: usize) -> Option<Match<'t>> {
        let start = (*self.slots.get(2 * i)?)?;
        let end = (*self.slots.get(2 * i + 1)?)?;
        Some(Match {
            text: self.text,
            start,
            end,
        })
    }

    /// The number of capture groups, **including** the overall match at index 0
    /// (so this is always ≥ 1). Mirrors `regex::Captures::len`.
    pub fn len(&self) -> usize {
        self.slots.len() / 2
    }

    /// Always `false` — a `Captures` always has at least the overall match (group
    /// 0). Present so `len` has its conventional companion.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// A single overall match: the byte range it spans in the searched text, and the
/// matched substring. Byte offsets (not char indices) match the `regex` crate and
/// index directly into the original `&str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<'t> {
    text: &'t str,
    start: usize,
    end: usize,
}

impl<'t> Match<'t> {
    /// Byte offset of the start of the match.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Byte offset just past the end of the match.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The matched substring.
    pub fn as_str(&self) -> &'t str {
        &self.text[self.start..self.end]
    }

    /// The match as a byte `Range`, suitable for slicing the searched text.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// Escape every regular-expression metacharacter in `text`, returning a pattern
/// that matches `text` literally. Mirrors `regex::escape` (same metacharacter
/// set as `regex-syntax`), so a glob/search-pattern builder that interleaves
/// escaped literals with wildcard fragments produces the same source string.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_meta_character(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The regex metacharacters `escape` guards, matching `regex-syntax`'s
/// `is_meta_character`. Escaping any of these makes it a literal; the parser
/// treats `\<char>` as a literal for all of them.
fn is_meta_character(c: char) -> bool {
    matches!(
        c,
        '\\' | '.'
            | '+'
            | '*'
            | '?'
            | '('
            | ')'
            | '|'
            | '['
            | ']'
            | '{'
            | '}'
            | '^'
            | '$'
            | '#'
            | '&'
            | '-'
            | '~'
    )
}

/// A builder for a [`Regex`] with configurable flags, mirroring the small part
/// of `regex::RegexBuilder` that Engram uses.
pub struct RegexBuilder<'a> {
    pattern: &'a str,
    case_insensitive: bool,
    dot_matches_new_line: bool,
}

impl<'a> RegexBuilder<'a> {
    /// Start building a regex for `pattern`.
    pub fn new(pattern: &'a str) -> Self {
        RegexBuilder {
            pattern,
            case_insensitive: false,
            dot_matches_new_line: false,
        }
    }

    /// Enable case-insensitive matching (Unicode simple case folding in Unicode
    /// mode; ASCII folding under `(?-u)`).
    pub fn case_insensitive(&mut self, yes: bool) -> &mut Self {
        self.case_insensitive = yes;
        self
    }

    /// Make `.` match newlines too (the `s` flag).
    pub fn dot_matches_new_line(&mut self, yes: bool) -> &mut Self {
        self.dot_matches_new_line = yes;
        self
    }

    /// Compile the configured pattern.
    pub fn build(&self) -> Result<Regex, Error> {
        let (tree, group_count, mut flags) =
            ast::parse(self.pattern).map_err(|e| Error(e.to_string()))?;
        // Builder flags OR the inline `(?…)` flags. `unicode` is carried straight
        // from the pattern's inline flags (default on; `(?-u)` turns it off).
        flags = Flags {
            case_insensitive: flags.case_insensitive || self.case_insensitive,
            dot_matches_new_line: flags.dot_matches_new_line || self.dot_matches_new_line,
            unicode: flags.unicode,
        };
        let program = program::compile(&tree, group_count, flags, MAX_PROGRAM_INSTS)
            .map_err(|e| Error(e.to_string()))?;
        Ok(Regex { program })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_neutralizes_metacharacters_and_round_trips() {
        // Every escaped metacharacter becomes a literal, so the escaped string
        // matches itself exactly and matches nothing broader.
        let raw = r"a.b*c+d?(e)|[f]{g}^h$#&-~i\j";
        let escaped = escape(raw);
        let re = Regex::new(&escaped).unwrap();
        assert!(
            re.is_match(raw),
            "escaped pattern must match its own literal"
        );
        // `.` is escaped, so it must NOT act as a wildcard.
        assert!(!Regex::new(&escape("a.c")).unwrap().is_match("axc"));
        assert!(Regex::new(&escape("a.c")).unwrap().is_match("a.c"));
        // A metacharacter-free string is returned unchanged.
        assert_eq!(escape("hello world 123"), "hello world 123");
    }

    fn find_span(pat: &str, text: &str) -> Option<(usize, usize)> {
        Regex::new(pat)
            .unwrap()
            .find(text)
            .map(|m| (m.start(), m.end()))
    }

    fn group_spans(pat: &str, text: &str) -> Vec<Option<(usize, usize)>> {
        let re = Regex::new(pat).unwrap();
        let caps = re.captures(text).unwrap();
        (0..caps.len())
            .map(|i| caps.get(i).map(|m| (m.start(), m.end())))
            .collect()
    }

    #[test]
    fn replace_all_with_closure() {
        // The shape Engram's media replace_all uses: a closure reading groups.
        let re = Regex::new(r"(\d+)").unwrap();
        let out = re.replace_all("a12b345c", |caps: &Captures| {
            format!("[{}]", caps.get(1).unwrap().as_str())
        });
        assert_eq!(out, "a[12]b[345]c");
        // No match ⇒ borrowed unchanged.
        let out = re.replace_all("abc", |_: &Captures| String::new());
        assert_eq!(out, "abc");
        assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn replace_all_with_string_refs() {
        let re = Regex::new(r"(\w+)@(\w+)").unwrap();
        assert_eq!(re.replace_all("x a@b y", "$2.$1"), "x b.a y");
        assert_eq!(re.replace_all("a@b", "${1}_${2}"), "a_b");
        // `$$` is a literal dollar; `$0` is the whole match.
        assert_eq!(re.replace_all("a@b", "$$$0"), "$a@b");
    }

    #[test]
    fn replace_all_empty_match_semantics() {
        // Matches `regex`: an empty-matching pattern inserts between chars without
        // doubling at the seams.
        assert_eq!(Regex::new("").unwrap().replace_all("ab", "-"), "-a-b-");
        assert_eq!(Regex::new("a*").unwrap().replace_all("aba", "-"), "-b-");
    }

    #[test]
    fn find_iter_and_captures_iter() {
        let re = Regex::new(r"\d+").unwrap();
        let spans: Vec<_> = re
            .find_iter("a1b22c333")
            .map(|m| m.as_str().to_string())
            .collect();
        assert_eq!(spans, ["1", "22", "333"]);
        let re = Regex::new(r"(\w)=(\d)").unwrap();
        let pairs: Vec<_> = re
            .captures_iter("x=1 y=2")
            .map(|c| {
                format!(
                    "{}{}",
                    c.get(1).unwrap().as_str(),
                    c.get(2).unwrap().as_str()
                )
            })
            .collect();
        assert_eq!(pairs, ["x1", "y2"]);
    }

    #[test]
    fn captures_reports_group_boundaries() {
        // Overall match + one group.
        assert_eq!(
            group_spans(r"(\d+)-(\d+)", "x12-345y"),
            vec![Some((1, 7)), Some((1, 3)), Some((4, 7))]
        );
        // A non-participating alternation branch is `None`.
        let caps = Regex::new(r"(a)|(b)").unwrap().captures("b").unwrap();
        assert_eq!(caps.get(0).map(|m| m.as_str()), Some("b"));
        assert_eq!(caps.get(1), None); // (a) did not participate
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("b"));
        // No match ⇒ None.
        assert!(Regex::new(r"(z)").unwrap().captures("abc").is_none());
    }

    #[test]
    fn captures_noncapturing_group_has_no_slot() {
        let caps = Regex::new(r"(?:ab)+(c)")
            .unwrap()
            .captures("ababc")
            .unwrap();
        assert_eq!(caps.len(), 2); // group 0 + the one capturing group
        assert_eq!(caps.get(1).map(|m| m.as_str()), Some("c"));
    }

    #[test]
    fn captures_last_iteration_wins_in_repeat() {
        // A quantified group captures its last iteration (matching `regex`).
        let caps = Regex::new(r"(\d)+").unwrap().captures("789").unwrap();
        assert_eq!(caps.get(0).map(|m| m.as_str()), Some("789"));
        assert_eq!(caps.get(1).map(|m| m.as_str()), Some("9"));
    }

    #[test]
    fn captures_media_pattern_shape() {
        // The shape of Engram's media-tag regex: disjoint quoted alternatives.
        let re = Regex::new(r#"<img src=(?:"([^"]+)"|'([^']+)')>"#).unwrap();
        let caps = re.captures(r#"<img src="a.png">"#).unwrap();
        assert_eq!(caps.get(1).map(|m| m.as_str()), Some("a.png"));
        assert_eq!(caps.get(2), None);
        let caps = re.captures(r#"<img src='b.gif'>"#).unwrap();
        assert_eq!(caps.get(1), None);
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("b.gif"));
    }

    #[test]
    fn too_many_groups_is_rejected() {
        let pat = "()".repeat(1001);
        assert!(Regex::new(&pat).is_err());
    }

    #[test]
    fn find_reports_leftmost_greedy_extent() {
        // Leftmost start, greedy end.
        assert_eq!(find_span("a+", "xaaay"), Some((1, 4)));
        assert_eq!(find_span("a+", "xaay aaaa"), Some((1, 3))); // leftmost, not longest-overall
                                                                // as_str reflects the span.
        let re = Regex::new(r"\d+").unwrap();
        assert_eq!(re.find("abc123def").unwrap().as_str(), "123");
        // No match.
        assert_eq!(find_span("z", "abc"), None);
    }

    #[test]
    fn find_lazy_vs_greedy() {
        assert_eq!(find_span("a+?", "aaaa"), Some((0, 1))); // lazy: as few as possible
        assert_eq!(find_span("a+", "aaaa"), Some((0, 4))); // greedy: as many as possible
    }

    #[test]
    fn find_empty_and_anchored() {
        assert_eq!(find_span("a?", "b"), Some((0, 0))); // empty match at start
        assert_eq!(find_span("", "xyz"), Some((0, 0))); // empty pattern
        assert_eq!(find_span("^abc", "zabc"), None); // anchored, no match past 0
        assert_eq!(find_span(r"abc$", "abc"), Some((0, 3)));
    }

    #[test]
    fn find_nullable_loops_match_greedily() {
        // The extent problem `find` must get right: a loop whose body can match
        // empty must still consume the whole greedy run, not stop at the empty
        // prefix. (Cross-checked against the `regex` crate in the integration test.)
        assert_eq!(find_span("(a?)*", "aaa"), Some((0, 3)));
        assert_eq!(find_span("(a*)*", "aaaa"), Some((0, 4)));
        assert_eq!(find_span("(a?)+", "aa"), Some((0, 2)));
        // A lazy nullable loop prefers the empty match.
        assert_eq!(find_span("(a??)+", "aa"), Some((0, 0)));
    }

    #[test]
    fn find_returns_byte_offsets_for_multibyte_text() {
        // "é" is 2 bytes, "😀" is 4 — offsets must be byte, not char, indices.
        let re = Regex::new("b").unwrap();
        let m = re.find("é😀b").unwrap();
        assert_eq!((m.start(), m.end()), (6, 7)); // 2 + 4 = 6 bytes precede 'b'
        assert_eq!(m.as_str(), "b");
    }

    #[test]
    fn literals_and_dot() {
        assert!(Regex::new("abc").unwrap().is_match("xxabcyy"));
        assert!(!Regex::new("abc").unwrap().is_match("abx"));
        assert!(Regex::new("a.c").unwrap().is_match("axc"));
        assert!(!Regex::new("a.c").unwrap().is_match("a\nc")); // `.` excludes \n
        assert!(Regex::new("(?s)a.c").unwrap().is_match("a\nc")); // dot-all
    }

    #[test]
    fn quantifiers() {
        assert!(Regex::new("ab*c").unwrap().is_match("ac"));
        assert!(Regex::new("ab+c").unwrap().is_match("abbbc"));
        assert!(!Regex::new("ab+c").unwrap().is_match("ac"));
        assert!(Regex::new("a{2,3}").unwrap().is_match("aaaa"));
        assert!(!Regex::new("^a{2,3}$").unwrap().is_match("a"));
    }

    #[test]
    fn classes_and_escapes() {
        assert!(Regex::new(r"\d+").unwrap().is_match("abc123"));
        assert!(Regex::new(r"[a-c]+").unwrap().is_match("zzbbc"));
        assert!(!Regex::new(r"[^a-c]").unwrap().is_match("a"));
        assert!(Regex::new(r"a\.b").unwrap().is_match("a.b"));
        assert!(!Regex::new(r"a\.b").unwrap().is_match("axb"));
    }

    #[test]
    fn anchors_and_boundaries() {
        assert!(Regex::new("^abc$").unwrap().is_match("abc"));
        assert!(!Regex::new("^abc$").unwrap().is_match("xabc"));
        assert!(Regex::new(r"\bword\b").unwrap().is_match("a word here"));
        assert!(!Regex::new(r"\bword\b").unwrap().is_match("keywords"));
    }

    #[test]
    fn alternation_and_groups() {
        assert!(Regex::new("cat|dog").unwrap().is_match("i have a dog"));
        assert!(Regex::new(r"(\d+)-(\d+)").unwrap().is_match("x 12-345 y"));
        assert!(!Regex::new(r"(\d+)-(\d+)").unwrap().is_match("nope"));
    }

    #[test]
    fn case_insensitive() {
        let re = RegexBuilder::new("hello")
            .case_insensitive(true)
            .build()
            .unwrap();
        assert!(re.is_match("HeLLo"));
        assert!(!Regex::new("hello").unwrap().is_match("HELLO"));
    }

    #[test]
    fn unicode_case_folding() {
        let ci = |p: &str| RegexBuilder::new(p).case_insensitive(true).build().unwrap();
        // Accented Latin folds both directions.
        assert!(ci("café").is_match("CAFÉ"));
        assert!(ci("CAFÉ").is_match("café"));
        // Greek final sigma: σ, ς and Σ are all in one fold orbit.
        assert!(ci("σ").is_match("ς"));
        assert!(ci("ς").is_match("Σ"));
        // Special simple folds a std upper/lower closure would miss.
        assert!(ci("k").is_match("\u{212A}")); // KELVIN SIGN
        assert!(ci("å").is_match("\u{212B}")); // ANGSTROM SIGN
        assert!(ci("s").is_match("ſ")); // LATIN SMALL LETTER LONG S
                                        // Inside a class, too.
        assert!(ci("[σ]").is_match("Σ"));
        // `(?-u)` reverts to ASCII folding: é/É no longer fold.
        assert!(!RegexBuilder::new("(?-u)é")
            .case_insensitive(true)
            .build()
            .unwrap()
            .is_match("É"));
    }

    #[test]
    fn over_large_pattern_is_rejected() {
        assert!(Regex::new("a{0,10000000}").is_err());
    }

    #[test]
    fn compile_path_dos_patterns_are_rejected_not_crashed() {
        // Regression guards for compile-time DoS (found in security review):
        // huge `{m,n}` bounds must be rejected at parse time, not expanded.
        assert!(Regex::new("a{0,4000000000}").is_err()); // would OOM if expanded
        assert!(Regex::new("a{4000000000}").is_err()); // would spin the min-loop
        assert!(Regex::new("a{4000000000,}").is_err());
        // Deeply nested groups must error, not overflow the stack.
        let deep = format!("{}a{}", "(".repeat(100_000), ")".repeat(100_000));
        assert!(Regex::new(&deep).is_err());
        // Sanity: a reasonable bound still works.
        assert!(Regex::new("a{2,5}").unwrap().is_match("aaa"));
    }

    #[test]
    fn unicode_classes_by_default() {
        // `\w` is Unicode by default → matches accented letters and CJK.
        assert!(Regex::new(r"^\w+$").unwrap().is_match("café"));
        assert!(Regex::new(r"^\w+$").unwrap().is_match("日本語"));
        // `(?-u)` reverts to ASCII → accented letters are NOT word chars.
        assert!(!Regex::new(r"(?-u)^\w+$").unwrap().is_match("café"));
        // Unicode digits.
        assert!(Regex::new(r"^\d+$").unwrap().is_match("١٢٣")); // Arabic-Indic
        assert!(!Regex::new(r"(?-u)^\d+$").unwrap().is_match("١٢٣"));
    }

    #[test]
    fn unicode_properties() {
        assert!(Regex::new(r"\p{Alphabetic}").unwrap().is_match("ω"));
        assert!(Regex::new(r"\p{Nd}").unwrap().is_match("५")); // Devanagari 5
        assert!(Regex::new(r"\p{Mark}").unwrap().is_match("a\u{0301}")); // combining acute
        assert!(!Regex::new(r"\p{Mark}").unwrap().is_match("abc"));
        // Negated property.
        assert!(Regex::new(r"^\P{Alphabetic}+$").unwrap().is_match("123 !"));
        // Unknown property is a compile error.
        assert!(Regex::new(r"\p{Nonsense}").is_err());
        // A class mixing a Unicode property and a literal.
        assert!(Regex::new(r"^[\p{Nd}_]+$").unwrap().is_match("5_٣"));
    }

    #[test]
    fn unicode_word_boundary() {
        // `\b` uses Unicode word chars by default: "café" is one word.
        assert!(Regex::new(r"\bcafé\b").unwrap().is_match("un café ici"));
        assert!(!Regex::new(r"\bcaf\b").unwrap().is_match("café")); // no boundary inside
    }

    #[test]
    fn no_catastrophic_backtracking() {
        // A pattern that destroys naive backtrackers; the Pike VM stays linear.
        let re = Regex::new("(a*)*b").unwrap();
        let input = "a".repeat(50);
        assert!(!re.is_match(&input)); // no trailing b
    }
}
