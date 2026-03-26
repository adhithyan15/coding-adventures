# Changelog — commonmark

## 0.1.0 — 2026-03-24

Initial release.

### Added

- `markdown_to_html(markdown: &str) -> String` — parse Markdown and render to HTML (raw HTML passthrough enabled)
- `markdown_to_html_safe(markdown: &str) -> String` — same but strips all raw HTML (safe for untrusted input)
- Re-exports: `parse` (from `commonmark-parser`), `to_html`, `escape_html`, `sanitize_url`, `normalize_url_for_attr`, `RenderOptions` (from `document-ast-to-html`), `document_ast`
- Integration test suite: 652/652 (100%) GFM 0.31.2 spec examples pass
- 27 section-level tests (one per GFM spec section, all passing)
- Unit tests and doctests
