# Changelog — commonmark (Go)

## [0.1.0] — 2026-03-24

### Added

- Initial release of the `commonmark` Go pipeline package.
- `Parse(markdown string) *documentast.DocumentNode` — parse Markdown to a
  Document AST (TE00).
- `ToHtml(markdown string) string` — parse and render Markdown to HTML with raw
  HTML passthrough enabled (full CommonMark spec compliance).
- `ToHtmlSafe(markdown string) string` — parse and render Markdown to HTML with
  all raw HTML stripped (safe for untrusted user content).
- `VERSION = "0.1.0"` and `COMMONMARK_VERSION = "0.31.2"` constants.
- `TestCommonMarkSpec` — loads the official CommonMark 0.31.2 `spec.json`
  (652 examples) and verifies all examples pass.
- `TestBasicParsing` — 13 hand-crafted test cases covering headings, paragraphs,
  emphasis, code blocks, fenced code, lists, blockquotes, inline links, thematic
  breaks, hard breaks, code spans, and empty documents.
- `TestSanitize` — verifies that `ToHtmlSafe` strips `<script>` tags.
- `spec.json` — official CommonMark 0.31.2 spec test suite (652 examples)
  shipped alongside the package for offline testing.
