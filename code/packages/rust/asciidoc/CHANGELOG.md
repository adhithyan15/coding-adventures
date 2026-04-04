# Changelog

## [0.1.0] - 2026-04-04

### Added
- Initial release of the AsciiDoc convenience crate for Rust.
- `asciidoc_to_html(text: &str) -> String` — converts AsciiDoc to HTML.
- `asciidoc_to_html_safe(text: &str) -> String` — converts AsciiDoc to HTML with raw HTML passthrough stripped.
- `parse(text: &str) -> DocumentNode` — re-exported from `asciidoc-parser`.
- `to_html(doc, opts)` and `RenderOptions` — re-exported from `document-ast-to-html`.
- Thin wrapper around `asciidoc-parser` and `document-ast-to-html`.
