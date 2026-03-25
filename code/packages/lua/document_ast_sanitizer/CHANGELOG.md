# Changelog — coding-adventures-document-ast-sanitizer

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `coding_adventures.document_ast_sanitizer`.
- `src/coding_adventures/document_ast_sanitizer/policy.lua` — `SanitizationPolicy` table
  and three named presets: `STRICT`, `RELAXED`, `PASSTHROUGH`.
- `src/coding_adventures/document_ast_sanitizer/url_utils.lua` — `strip_control_chars()`,
  `extract_scheme()`, `is_scheme_allowed()` — defends against C0 control char and
  zero-width space URL bypass vectors.
- `src/coding_adventures/document_ast_sanitizer/sanitizer.lua` — pure, immutable
  `sanitize(doc, policy)` function implementing the full TE02 truth table for all
  Document AST node types.
- `src/coding_adventures/document_ast_sanitizer/init.lua` — public module entry point
  re-exporting all public API symbols.
- `spec/sanitizer_spec.lua` — 85 busted tests covering every policy option,
  all XSS bypass vectors from the TE02 spec, immutability guarantee, and empty-child pruning.

### Policy features implemented

- `allowRawBlockFormats` / `allowRawInlineFormats` — `"drop-all"`, `"passthrough"`, or format allowlist
- `allowedUrlSchemes` — scheme allowlist for links, images, and autolinks
- `dropLinks` — link child promotion (text preserved, `<a>` removed)
- `dropImages` / `transformImageToText` — drop or convert images to alt text
- `maxHeadingLevel` / `minHeadingLevel` — clamp or drop heading levels
- `dropBlockquotes` / `dropCodeBlocks` — structural element removal
- `transformCodeSpanToText` — code span to plain text conversion

### Test coverage

- 85 tests, 0 failures
- Covers all node types in the Document AST truth table
- XSS vectors: `javascript:`, `JAVASCRIPT:`, `java\x00script:`, zero-width space bypass
- Control character stripping verified for all C0 bytes and Unicode invisible chars
