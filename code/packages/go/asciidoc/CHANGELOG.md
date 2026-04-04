# Changelog

## [0.1.0] - 2026-04-04

### Added
- Initial release of the AsciiDoc convenience wrapper for Go.
- `ToHtml(text string) string` — converts AsciiDoc to HTML.
- `ToHtmlSafe(text string) string` — converts AsciiDoc to HTML with raw HTML passthrough stripped.
- `Parse(text string) *DocumentNode` — parses AsciiDoc to Document AST.
- Thin wrapper around `asciidoc-parser` and `document-ast-to-html`.
