# Changelog

All notable changes to the `coding-adventures-jsdoc-comment-extractor` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC05 §"jsdoc-comment-extractor."
- `extract_block_comments(source: &str) -> Vec<BlockComment>` — pure byte scan over a JS source string. Returns comments in source order with non-overlapping spans.
- `BlockComment { span: Span, inner: String, anchor_byte: u32 }`:
  - `span` covers the full `/**` … `*/` range.
  - `inner` is the cleaned body with `/**`/`*/` markers and per-line `* ` (or `*`) continuation prefixes stripped; leading and trailing whitespace trimmed.
  - `anchor_byte` mirrors `span.start` in v1; will become "anchored AST node start byte" once the AST integration follow-up lands.
- Behavior rules pinned by tests:
  - Single-asterisk block comments `/* ... */` and the empty `/**/ ` are NOT JSDoc and are skipped.
  - Triple-asterisk openers (`/*** ... */`) are accepted (matching jsdoc.app + TypeScript checkJs); the third `*` becomes part of the inner body.
  - Unterminated `/**` stops scanning gracefully (no panic; lexer-level error surfacing is a follow-up concern).
  - Multi-paragraph comments preserve their blank lines.
- 13 tests covering: empty source, no-comment source, single-line tag, multi-line tag with continuation prefix, multi-comment ordering, empty JSDoc `/***/ `, non-JSDoc `/* */` skipped, empty `/**/ ` skipped, triple-star open, unterminated graceful failure, tab-prefixed continuation, false-positive inside string (documented limitation), `anchor_byte` matches `span.start`, blank line preservation.

### Notes
- v1 dependency whitelist: just `coding-adventures-javascript-tokens` (for `Span`). No regex crate, no `serde`, no `correlation-vector` (that integration lands with the AST-driven extractor).
- Documented limitations: no `javascript-ast::Program` integration yet (raw source only); no string-literal awareness so a `/** */` *inside* a JS string is reported as a real comment. Both fixed by the AST-driven follow-up.
