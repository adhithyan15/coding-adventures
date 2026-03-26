# Changelog — CodingAdventures.DocumentHtmlSanitizer

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `CodingAdventures.DocumentHtmlSanitizer.sanitize_html/2`
- `Policy` struct with three presets: `html_strict/0`, `html_relaxed/0`, `html_passthrough/0`
- `UrlUtils` module: URL scheme checking with control-character bypass prevention
  (independent copy — no shared dependency with document_ast_sanitizer)
- `HtmlSanitizer` module implementing a 5-step pipeline:
  1. Comment stripping (when `drop_comments: true`)
  2. Element dropping with content (case-insensitive, regex-based)
  3. Attribute sanitization: `on*` event handlers, `srcdoc`, `formaction`,
     plus any policy-specified `drop_attributes`
  4. URL attribute sanitization: `href` and `src` scheme validation
  5. Style attribute sanitization: CSS `expression()` and dangerous `url()` removal
- 76 ExUnit tests covering all XSS attack vector categories from the spec
- 94.23% test coverage
- Zero dependencies — Elixir stdlib only

### Notes

- `on*` event handler stripping is baseline security, applied for all policies
  including `html_passthrough`. This is intentional: the passthrough preset
  disables element dropping and URL/style sanitization, but not event handlers.
