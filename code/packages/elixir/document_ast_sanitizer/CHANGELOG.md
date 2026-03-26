# Changelog — CodingAdventures.DocumentAstSanitizer

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `CodingAdventures.DocumentAstSanitizer.sanitize/2`
- `Policy` struct with three presets: `strict/0`, `relaxed/0`, `passthrough/0`
- `UrlUtils` module: `strip_control_chars/1`, `extract_scheme/1`, `scheme_allowed?/2`
  - Strips C0 control characters (U+0000–U+001F) and zero-width characters
    (U+200B, U+200C, U+200D, U+2060, U+FEFF) before scheme extraction to
    block `java\x00script:` and similar bypasses
- Full truth-table implementation for all Document AST node types:
  - Block nodes: document, heading (with level clamping), paragraph, code_block,
    blockquote, list, list_item, thematic_break, raw_block
  - Inline nodes: text, emphasis, strong, code_span, link (with child promotion),
    image (with text conversion), autolink, raw_inline, hard_break, soft_break
- Empty-children pruning: container nodes with no surviving children are dropped
  (except document root which is always kept)
- 107 ExUnit tests covering all policy options and XSS attack vectors
- 97.26% test coverage
