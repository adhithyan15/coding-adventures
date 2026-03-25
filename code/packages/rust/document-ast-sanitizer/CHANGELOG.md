# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-24

### Added

- Initial implementation of `coding_adventures_document_ast_sanitizer` per TE02 spec
- `SanitizationPolicy` struct with all fields from the spec: `allow_raw_block_formats`,
  `allow_raw_inline_formats`, `allowed_url_schemes`, `drop_links`, `drop_images`,
  `transform_image_to_text`, `min_heading_level`, `max_heading_level`, `drop_blockquotes`,
  `drop_code_blocks`, `transform_code_span_to_text`
- `RawFormatPolicy` enum: `DropAll`, `Passthrough`, `Allowlist(Vec<String>)`
- `MaxHeadingLevel` enum: `Drop`, `Level(u8)`
- Named preset constructors: `strict()`, `relaxed()`, `passthrough()`
- Named preset constants: `STRICT`, `RELAXED`, `PASSTHROUGH` (with `allowed_url_schemes: None`
  — use the constructor functions for full scheme-restricted versions)
- `sanitize(doc: &DocumentNode, policy: &SanitizationPolicy) -> DocumentNode` — pure, immutable
  tree transformation covering all 24 node types from the Document AST
- URL scheme sanitization with C0 control char and zero-width char stripping to prevent
  bypass attacks (`java\x00script:`, `\u200Bjavascript:`, etc.)
- Empty-child pruning: container nodes whose children are all dropped are themselves dropped
  (except `DocumentNode` which is always kept)
- Link child promotion: when `drop_links: true`, link text children are promoted to the parent
  container as plain inline nodes
- 69 unit tests + 11 doc-tests covering all policy options and XSS vectors
- Full literate-programming doc comments throughout
