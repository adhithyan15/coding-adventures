# Changelog

## [0.1.0] - 2026-03-25

### Added

- Initial implementation of `commonmark-native` Python extension
- `markdown_to_html(markdown: str) -> str` — full CommonMark 0.31.2 pipeline
  with raw HTML passthrough for trusted author content
- `markdown_to_html_safe(markdown: str) -> str` — safe variant that strips
  all raw HTML blocks and inline HTML to prevent XSS attacks
- Zero-dependency implementation via `python-bridge` FFI (no PyO3, no bindgen)
- Cross-platform support: Linux (`.so`), macOS (`.dylib`), Windows (`.pyd`)
- Comprehensive test suite covering all CommonMark block and inline elements:
  headings, paragraphs, emphasis, strong, code spans, fenced code blocks,
  blockquotes, unordered/ordered lists, links, images, and raw HTML handling
- `__init__.py` re-exports for clean `from commonmark_native import ...` usage
