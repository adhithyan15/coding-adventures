# Changelog

All notable changes to the `coding-adventures-javascript-tokens` crate will be documented in this file.

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
