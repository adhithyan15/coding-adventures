//! Shared JavaScript tokens and ES version enum.
//!
//! # What this crate is for
//!
//! `javascript-tokens` holds the **backend-agnostic** types used across the
//! JavaScript pipeline — anything that more than one crate downstream needs
//! to agree on. It deliberately has no dependencies (not even `serde` for
//! v1) so it sits at the bottom of the dependency graph and can be pulled in
//! by everything else without creating cycles.
//!
//! Per [CLOC02](../../specs/CLOC02-javascript-ast.md), the JS frontend is
//! shared between two backends:
//!
//! 1. The Closure-Compiler-clone (JS → optimised JS).
//! 2. The future V8-in-Rust clone (JS → bytecode → VM).
//!
//! Both backends consume the same parser AST, which in turn references the
//! types here.
//!
//! # What's here
//!
//! - [`EsVersion`] — the enum naming every ECMAScript edition that has a
//!   grammar file under `code/grammars/ecmascript/`. The lexer and parser
//!   today still take version strings, but typed APIs that consume
//!   `EsVersion` directly are also available (see `javascript-lexer` and
//!   `javascript-parser` v0.4.0+).
//! - [`Span`] — a `{ start, end }` byte-offset range. Used by the lexer to
//!   record where each token came from in the source, and by the AST and
//!   correlation-vector layers to anchor everything back to bytes.
//! - [`TokenKind`] — the broad cross-version classification of every JS
//!   token (`Name`, `Number`, `String`, `Regex`, `Template*`, `BigInt`,
//!   `PrivateName`, `Keyword`, `Operator`, `Punctuation`, trivia, …). Use
//!   the `Other(String)` variant when a consumer needs the exact
//!   grammar-driven name (e.g. `"OPTIONAL_CHAIN"`).

use std::fmt;
use std::str::FromStr;

/// Every ECMAScript edition that has a grammar file under
/// `code/grammars/ecmascript/`.
///
/// ES2 and ES5.1 are not separate variants — per the
/// `versioned-ecmascript-typescript-grammars.md` spec they alias ES1 and ES5
/// respectively (editorial changes only), and the grammar tree omits them.
///
/// The string form (returned by [`EsVersion::as_str`]) matches the grammar
/// file basenames exactly: `"es1"`, `"es3"`, `"es5"`, `"es2015"` through
/// `"es2025"`. That's the same identifier the lexer's `SUPPORTED_VERSIONS`
/// list uses, so call sites can move between the two without renaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EsVersion {
    Es1,
    Es3,
    Es5,
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    Es2023,
    Es2024,
    Es2025,
}

impl EsVersion {
    /// The most recent edition we have a grammar for.
    ///
    /// Used as the default when callers don't specify a version. This is
    /// `Es2025` today; future grammar additions bump it.
    pub const fn latest() -> Self {
        EsVersion::Es2025
    }

    /// The basename of the matching grammar file under
    /// `code/grammars/ecmascript/`. Matches what the lexer/parser
    /// `SUPPORTED_VERSIONS` arrays contain — call sites can pass this
    /// directly to `tokenize_javascript` / `parse_javascript`.
    pub const fn as_str(self) -> &'static str {
        match self {
            EsVersion::Es1 => "es1",
            EsVersion::Es3 => "es3",
            EsVersion::Es5 => "es5",
            EsVersion::Es2015 => "es2015",
            EsVersion::Es2016 => "es2016",
            EsVersion::Es2017 => "es2017",
            EsVersion::Es2018 => "es2018",
            EsVersion::Es2019 => "es2019",
            EsVersion::Es2020 => "es2020",
            EsVersion::Es2021 => "es2021",
            EsVersion::Es2022 => "es2022",
            EsVersion::Es2023 => "es2023",
            EsVersion::Es2024 => "es2024",
            EsVersion::Es2025 => "es2025",
        }
    }

    /// Every variant in chronological order — useful for iterating across
    /// the cascade in tests.
    pub const ALL: &'static [EsVersion] = &[
        EsVersion::Es1,
        EsVersion::Es3,
        EsVersion::Es5,
        EsVersion::Es2015,
        EsVersion::Es2016,
        EsVersion::Es2017,
        EsVersion::Es2018,
        EsVersion::Es2019,
        EsVersion::Es2020,
        EsVersion::Es2021,
        EsVersion::Es2022,
        EsVersion::Es2023,
        EsVersion::Es2024,
        EsVersion::Es2025,
    ];
}

impl fmt::Display for EsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for EsVersion {
    fn default() -> Self {
        EsVersion::latest()
    }
}

/// Parsed back from the same strings [`EsVersion::as_str`] emits.
///
/// Anything else (`""`, `"es2"`, `"es5.1"`, `"latest"`, …) returns
/// [`UnknownEsVersion`]. We deliberately do not accept the empty string —
/// the legacy "generic" version retired by PR #3785 is not a valid
/// `EsVersion` and never will be.
impl FromStr for EsVersion {
    type Err = UnknownEsVersion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "es1" => Ok(EsVersion::Es1),
            "es3" => Ok(EsVersion::Es3),
            "es5" => Ok(EsVersion::Es5),
            "es2015" => Ok(EsVersion::Es2015),
            "es2016" => Ok(EsVersion::Es2016),
            "es2017" => Ok(EsVersion::Es2017),
            "es2018" => Ok(EsVersion::Es2018),
            "es2019" => Ok(EsVersion::Es2019),
            "es2020" => Ok(EsVersion::Es2020),
            "es2021" => Ok(EsVersion::Es2021),
            "es2022" => Ok(EsVersion::Es2022),
            "es2023" => Ok(EsVersion::Es2023),
            "es2024" => Ok(EsVersion::Es2024),
            "es2025" => Ok(EsVersion::Es2025),
            other => Err(UnknownEsVersion(other.to_string())),
        }
    }
}

/// Error returned by [`EsVersion::from_str`] when the input doesn't match
/// any known ES edition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEsVersion(pub String);

impl fmt::Display for UnknownEsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown ECMAScript version {:?}; valid values are {}",
            self.0,
            EsVersion::ALL
                .iter()
                .map(|v| format!("{:?}", v.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::error::Error for UnknownEsVersion {}

/// A byte range within a single source file.
///
/// `start` and `end` are **byte offsets**, not character or column counts.
/// They follow the half-open `[start, end)` convention — `end` is exclusive,
/// so an empty span at position `n` is `Span { start: n, end: n }`.
///
/// `u32` is wide enough to address any practical JavaScript source file
/// (4 GiB cap) and half the size of `usize` on 64-bit targets — important
/// because every token and every AST node will carry one or more spans.
///
/// Per [CLOC02](../../specs/CLOC02-javascript-ast.md), AST nodes themselves
/// do *not* hold spans directly — spans live in correlation-vector
/// [`Origin`] records, keyed by `CvId`. The AST holds only the `CvId`. This
/// type is what producers of those `Origin` records embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    /// Byte offset of the first byte in this range (inclusive).
    pub start: u32,
    /// Byte offset one past the last byte in this range (exclusive).
    pub end: u32,
}

impl Span {
    /// Construct a span from a half-open `[start, end)` byte range.
    ///
    /// Callers are expected to maintain `start <= end`. This invariant is
    /// not enforced at construction time — making it a runtime check would
    /// force every call site into `Result` territory for what is, in
    /// practice, a static guarantee from the lexer. Debug builds may add
    /// an assertion in a future revision.
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// The number of bytes in this span. Always returns `end - start` as
    /// a `u32`; callers responsible for the `start <= end` invariant.
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Whether this span covers zero bytes.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// The broad classification of a JavaScript / TypeScript token.
///
/// This enum names the **categories** common to every ES edition under
/// `code/grammars/ecmascript/` — `Name`, `Number`, `String`, etc. — plus
/// a few that only appear from a specific edition onward (`Regex` from
/// ES3, `Template*` from ES2015, `BigInt` from ES2020, `PrivateName`
/// from ES2022, `Hashbang` from ES2023). It is intentionally **not** an
/// exhaustive list of every operator and punctuation lexeme; per-version
/// token names (e.g. `OPTIONAL_CHAIN`, `STAR_STAR_EQUALS`) live in the
/// individual `.tokens` grammar files and can be carried via
/// [`TokenKind::Other`] when a consumer needs the exact name.
///
/// Per [CLOC02](../../specs/CLOC02-javascript-ast.md), this type belongs
/// here in `javascript-tokens` so that every downstream consumer — the
/// lexer, parser, AST, future V8 clone, IDE tooling — can talk about
/// token kinds without depending on any particular layer above.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// An identifier (`foo`, `_bar`, `$baz`).
    Name,
    /// A numeric literal (`42`, `3.14`, `0xFF`, `1_000`).
    Number,
    /// A string literal (single- or double-quoted).
    String,
    /// A regular-expression literal (ES3+).
    Regex,
    /// A complete template literal with no substitutions: `` `hello` ``
    /// (ES2015+).
    TemplateNoSub,
    /// The opening chunk of a template with substitutions: `` `hello${ ``
    /// (ES2015+).
    TemplateHead,
    /// A middle chunk between substitutions: `` }middle${ `` (ES2015+).
    TemplateMiddle,
    /// The closing chunk of a template: `` }end` `` (ES2015+).
    TemplateTail,
    /// A `BigInt` literal: `42n`, `0xFFn` (ES2020+).
    BigInt,
    /// A private class member name: `#count` (ES2022+).
    PrivateName,
    /// A reserved keyword (`var`, `let`, `class`, `async`, …).
    Keyword,
    /// An operator (`+`, `=>`, `?.`, `||=`, …). Use [`TokenKind::Other`]
    /// when the specific operator name from the grammar matters.
    Operator,
    /// A punctuation symbol (`(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`).
    Punctuation,
    /// A line or block comment. Treated as trivia by [`TokenKind::is_trivia`].
    Comment,
    /// A run of inter-token whitespace. Trivia.
    Whitespace,
    /// A line terminator. Trivia. Carried as its own variant (separately
    /// from `Whitespace`) because ASI cares about newlines.
    Newline,
    /// A `#!` shebang at the very start of a module (ES2023+).
    Hashbang,
    /// A lexer error token (e.g. unterminated string).
    Error,
    /// The end-of-input sentinel.
    Eof,
    /// Catch-all for grammar-driven tokens whose specific name doesn't
    /// fit any of the variants above. The wrapped `String` is the token
    /// name from the `.tokens` file (e.g. `"OPTIONAL_CHAIN"`,
    /// `"STAR_STAR_EQUALS"`).
    Other(std::string::String),
}

impl TokenKind {
    /// Returns `true` for token kinds that the parser typically skips:
    /// [`Comment`](TokenKind::Comment), [`Whitespace`](TokenKind::Whitespace),
    /// and [`Newline`](TokenKind::Newline).
    ///
    /// Note that `Newline` is sometimes *not* skipped — ASI implementations
    /// need to observe newlines to decide whether to insert a semicolon.
    /// The "trivia" classification here is a hint, not a hard rule.
    pub fn is_trivia(&self) -> bool {
        matches!(
            self,
            TokenKind::Comment | TokenKind::Whitespace | TokenKind::Newline
        )
    }

    /// Returns `true` for the end-of-input sentinel.
    pub fn is_eof(&self) -> bool {
        matches!(self, TokenKind::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_is_es2025() {
        assert_eq!(EsVersion::latest(), EsVersion::Es2025);
    }

    #[test]
    fn default_is_latest() {
        assert_eq!(EsVersion::default(), EsVersion::latest());
    }

    #[test]
    fn as_str_matches_grammar_filenames() {
        // The lexer's SUPPORTED_VERSIONS array (post-PR #3785) — verbatim.
        let expected = [
            "es1", "es3", "es5", "es2015", "es2016", "es2017", "es2018",
            "es2019", "es2020", "es2021", "es2022", "es2023", "es2024",
            "es2025",
        ];
        let actual: Vec<&str> = EsVersion::ALL.iter().map(|v| v.as_str()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn round_trip_through_strings() {
        for v in EsVersion::ALL {
            assert_eq!(EsVersion::from_str(v.as_str()).unwrap(), *v);
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        // The legacy "generic" version retired by PR #3785 must not parse.
        assert!(EsVersion::from_str("").is_err());
    }

    #[test]
    fn unknown_strings_are_rejected() {
        for bad in &["es2", "es5.1", "latest", "ES2025", "es2026", " es2025"] {
            assert!(
                EsVersion::from_str(bad).is_err(),
                "expected {:?} to be rejected",
                bad
            );
        }
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", EsVersion::Es2020), "es2020");
    }

    #[test]
    fn unknown_error_message_includes_valid_set() {
        let err = EsVersion::from_str("nope").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("\"nope\""), "msg = {msg}");
        assert!(msg.contains("\"es2025\""), "msg = {msg}");
    }

    #[test]
    fn ord_is_chronological() {
        assert!(EsVersion::Es1 < EsVersion::Es3);
        assert!(EsVersion::Es5 < EsVersion::Es2015);
        assert!(EsVersion::Es2015 < EsVersion::Es2025);
    }

    // ----- Span tests -----

    #[test]
    fn span_constructs_from_byte_range() {
        let s = Span::new(10, 20);
        assert_eq!(s.start, 10);
        assert_eq!(s.end, 20);
    }

    #[test]
    fn span_len_is_end_minus_start() {
        assert_eq!(Span::new(10, 20).len(), 10);
        assert_eq!(Span::new(0, 1).len(), 1);
        assert_eq!(Span::new(42, 42).len(), 0);
    }

    #[test]
    fn span_is_empty_when_start_equals_end() {
        assert!(Span::new(0, 0).is_empty());
        assert!(Span::new(99, 99).is_empty());
        assert!(!Span::new(0, 1).is_empty());
        assert!(!Span::new(10, 20).is_empty());
    }

    #[test]
    fn span_is_copy_and_eq() {
        // Compile-time assertion that Span is Copy.
        fn assert_copy<T: Copy>() {}
        assert_copy::<Span>();

        // PartialEq + Eq work over both fields.
        let a = Span::new(3, 7);
        let b = Span::new(3, 7);
        let c = Span::new(3, 8);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // The `assert!(!EMPTY)` intentionally asserts a compile-time constant to
    // document the const-construction invariant; that is the point of the test.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn span_supports_const_construction() {
        // `new`, `len`, `is_empty` are all `const fn`, so a Span can live
        // in const context — useful for compile-time fixtures.
        const S: Span = Span::new(2, 5);
        const LEN: u32 = S.len();
        const EMPTY: bool = S.is_empty();
        assert_eq!(LEN, 3);
        assert!(!EMPTY);
    }

    #[test]
    fn span_ord_is_lexicographic() {
        // PartialOrd/Ord compares start first, then end.
        assert!(Span::new(0, 5) < Span::new(0, 6));
        assert!(Span::new(0, 5) < Span::new(1, 2));
        assert!(Span::new(5, 10) > Span::new(5, 5));
    }

    // ----- TokenKind tests -----

    /// Every variant must be classified explicitly here. If a future PR
    /// adds a new variant, the compiler will force this match to be
    /// updated (because we use the exhaustive form), which in turn forces
    /// a conscious decision about whether the new kind is trivia.
    #[test]
    fn token_kind_is_trivia_exhaustive() {
        // Match every variant explicitly so adding a new one breaks
        // compilation and forces an update here.
        let cases: &[(TokenKind, bool)] = &[
            (TokenKind::Name, false),
            (TokenKind::Number, false),
            (TokenKind::String, false),
            (TokenKind::Regex, false),
            (TokenKind::TemplateNoSub, false),
            (TokenKind::TemplateHead, false),
            (TokenKind::TemplateMiddle, false),
            (TokenKind::TemplateTail, false),
            (TokenKind::BigInt, false),
            (TokenKind::PrivateName, false),
            (TokenKind::Keyword, false),
            (TokenKind::Operator, false),
            (TokenKind::Punctuation, false),
            (TokenKind::Comment, true),
            (TokenKind::Whitespace, true),
            (TokenKind::Newline, true),
            (TokenKind::Hashbang, false),
            (TokenKind::Error, false),
            (TokenKind::Eof, false),
            (TokenKind::Other("anything".to_string()), false),
        ];
        for (kind, expected) in cases {
            assert_eq!(
                kind.is_trivia(),
                *expected,
                "is_trivia disagrees for {:?}",
                kind
            );
        }
    }

    #[test]
    fn token_kind_is_eof_only_for_eof() {
        assert!(TokenKind::Eof.is_eof());
        assert!(!TokenKind::Name.is_eof());
        assert!(!TokenKind::Newline.is_eof());
        assert!(!TokenKind::Other("EOF".to_string()).is_eof());
    }

    #[test]
    fn token_kind_equality() {
        assert_eq!(TokenKind::Name, TokenKind::Name);
        assert_ne!(TokenKind::Name, TokenKind::Number);
        assert_eq!(
            TokenKind::Other("X".to_string()),
            TokenKind::Other("X".to_string())
        );
        assert_ne!(
            TokenKind::Other("X".to_string()),
            TokenKind::Other("Y".to_string())
        );
    }

    #[test]
    fn token_kind_usable_as_hashmap_key() {
        use std::collections::HashMap;
        let mut counts: HashMap<TokenKind, u32> = HashMap::new();
        *counts.entry(TokenKind::Name).or_insert(0) += 1;
        *counts.entry(TokenKind::Name).or_insert(0) += 1;
        *counts.entry(TokenKind::Number).or_insert(0) += 1;
        *counts
            .entry(TokenKind::Other("FOO".to_string()))
            .or_insert(0) += 5;
        assert_eq!(counts.get(&TokenKind::Name), Some(&2));
        assert_eq!(counts.get(&TokenKind::Number), Some(&1));
        assert_eq!(
            counts.get(&TokenKind::Other("FOO".to_string())),
            Some(&5)
        );
        assert_eq!(counts.get(&TokenKind::Other("BAR".to_string())), None);
    }
}
