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
//! selects the ASCII sets. `(?i)` uses Unicode simple case folding. Not yet
//! included: match *extents* (`find`/`captures`/`replace_all`).

mod ast;
mod casefold;
mod program;
mod unicode_tables;

use ast::Flags;
use program::{Input, Program};

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
    ///
    /// Reporting the *extent* of a match (`find`/`captures`/`replace_all`) is not
    /// yet exposed: getting the exact match boundaries right for *nullable loops*
    /// (e.g. `(a?)*`) is a distinct sub-problem, and Engram needs match extents
    /// only for one fixed pattern (the media-tag replacement), which is added in
    /// a later, separately-verified change.
    pub fn is_match(&self, text: &str) -> bool {
        let input = Input::new(text);
        self.program.is_match_from(&input, 0)
    }
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
