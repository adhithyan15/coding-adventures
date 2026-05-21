# Changelog

All notable changes to the `coding-adventures-javascript-tokens` crate will be documented in this file.

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
