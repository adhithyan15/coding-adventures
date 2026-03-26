# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-24

### Added

- Initial implementation of `coding_adventures_document_html_sanitizer` per TE02 spec
- `HtmlSanitizationPolicy` struct with all fields from the spec: `drop_elements`,
  `drop_attributes`, `allowed_url_schemes`, `drop_comments`, `sanitize_style_attributes`
- Named preset constructors: `html_strict()`, `html_relaxed()`, `html_passthrough()`
- `sanitize_html(html: &str, policy: &HtmlSanitizationPolicy) -> String` — regex-based
  HTML string sanitizer with no dependency on document-ast
- Element removal with content: `<script>…</script>`, `<style>…</style>`, etc. removed
  entirely including body content
- Void element removal: `<meta …>`, `<input …>`, `<link …>` without closing tags
- Always-on `on*` event handler attribute stripping (independent of policy)
- `srcdoc` and `formaction` attribute stripping in `html_strict()`
- URL scheme sanitization for `href` and `src` attributes using the same
  control-char stripping algorithm as the AST sanitizer
- CSS injection prevention: `expression(` in style attributes stripped entirely
- `url(javascript:…)` in style attributes stripped (allows `url(https://…)`)
- HTML comment stripping including IE conditional comments
- Multi-line element and comment handling via regex `(?s)` dot-matches-newline flag
- Pre-compiled regex patterns via `OnceLock` for performance
- 44 unit tests + 7 doc-tests covering all XSS vectors from the spec
- Full literate-programming doc comments throughout
