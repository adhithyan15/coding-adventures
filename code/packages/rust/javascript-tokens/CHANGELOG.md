# Changelog

All notable changes to the `coding-adventures-javascript-tokens` crate will be documented in this file.

## [0.3.0] - 2026-05-21

### Added
- `TokenKind` enum: broad cross-version classification of JS/TS tokens per CLOC02. Variants: `Name`, `Number`, `String`, `Regex`, `TemplateNoSub`, `TemplateHead`, `TemplateMiddle`, `TemplateTail`, `BigInt`, `PrivateName`, `Keyword`, `Operator`, `Punctuation`, `Comment`, `Whitespace`, `Newline`, `Hashbang`, `Error`, `Eof`, plus `Other(String)` as the catch-all for grammar-driven names (e.g. `"OPTIONAL_CHAIN"`, `"STAR_STAR_EQUALS"`).
- Derives: `Debug, Clone, PartialEq, Eq, Hash` — `Hash` lets `TokenKind` serve as a `HashMap` key for per-kind statistics.
- `TokenKind::is_trivia() -> bool` — `true` for `Comment`, `Whitespace`, `Newline`. Documented as a hint (ASI may need to observe newlines).
- `TokenKind::is_eof() -> bool` — `true` for `Eof` only.
- Module-level docs and README updated to introduce `TokenKind`.
- 4 new tests covering the type:
  - `token_kind_is_trivia_exhaustive` — every variant has an explicit row in the table; adding a new variant in a future PR forces this test to be updated (compile-time enforcement via exhaustive listing).
  - `token_kind_is_eof_only_for_eof`.
  - `token_kind_equality` — including `Other` variants compare by inner string.
  - `token_kind_usable_as_hashmap_key`.

### Notes
- Still zero runtime dependencies.
- This is deliberately **not** the full per-version token enum (no `PlusEquals`, `OptionalChain`, etc. as named variants). Per-version names round-trip through `Other(String)` until the day a consumer needs strongly-typed operator categories — that's a follow-up.

## [0.2.0] - 2026-05-21

### Added
- `Span { start: u32, end: u32 }` byte-offset range type per CLOC02. Half-open `[start, end)` semantics. `u32` chosen for cache-friendly node sizes; supports any practical source file size.
- `Span::new(start, end) -> Self` — `const fn` constructor (callers maintain `start <= end` invariant).
- `Span::len(self) -> u32` and `Span::is_empty(self) -> bool` — both `const fn`.
- Derives: `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord` — Ord is lexicographic on `(start, end)`.
- Module-level docs updated to introduce `Span`.
- 6 new tests: construction, len, is_empty edge cases (zero-length, non-zero), `Copy` + `PartialEq`, `const fn` usage in const context, lexicographic `Ord`.

### Notes
- Still zero runtime dependencies — `Span` is just two `u32`s.
- Per CLOC02, AST nodes do NOT hold spans directly; spans live in correlation-vector `Origin` records, keyed by `CvId`. This type is what `Origin` producers (the lexer) embed.
- A full `TokenKind` enum is still pending — it's much larger and gets its own PR.

## [0.1.0] - 2026-05-21

### Added
- New crate scaffolded per CLOC02 Phase 1.
- `EsVersion` enum: every ECMAScript edition with a grammar file under `code/grammars/ecmascript/` (`Es1`, `Es3`, `Es5`, `Es2015` through `Es2025`).
- `EsVersion::latest()` returning the most recent edition (currently `Es2025`).
- `EsVersion::as_str()` returning the basename string matching the grammar files (`"es1"`, `"es3"`, `"es5"`, `"es2015"` through `"es2025"`) — interoperates directly with `javascript-lexer`'s `SUPPORTED_VERSIONS`.
- `EsVersion::ALL` constant slice for iteration across the cascade.
- `Default for EsVersion` returning `latest()`.
- `Display for EsVersion` delegating to `as_str()`.
- `FromStr for EsVersion` accepting only the canonical strings from `as_str()`. Empty string and the legacy "generic" version (retired by PR #3785) are rejected.
- `UnknownEsVersion` error type with a descriptive `Display` message listing the valid set.
- Test suite covering: `latest()`, `Default`, string round-trip, empty-string rejection, unknown-string rejection, `Display`, error-message content, chronological `Ord`.

### Notes
- No dependencies (not even `serde`); the crate is the bottom of the JS pipeline's dependency graph.
- `TokenKind` and `Span` types from CLOC02 ship in follow-up PRs.
