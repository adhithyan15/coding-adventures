# Changelog — coding-adventures-document-html-sanitizer

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `coding_adventures.document_html_sanitizer`.
- `src/coding_adventures/document_html_sanitizer/policy.lua` — `HtmlSanitizationPolicy`
  table and three named presets: `HTML_STRICT`, `HTML_RELAXED`, `HTML_PASSTHROUGH`.
- `src/coding_adventures/document_html_sanitizer/url_utils.lua` — independent copy of
  URL utility functions (no dependency on `document_ast_sanitizer`). Strips C0 controls
  and Unicode invisible code points before scheme extraction.
- `src/coding_adventures/document_html_sanitizer/html_sanitizer.lua` — multi-pass
  pattern-based `sanitize_html(html, policy)` function.
- `src/coding_adventures/document_html_sanitizer/init.lua` — public module entry point.
- `spec/html_sanitizer_spec.lua` — 54 busted tests covering all XSS categories from
  the TE02 spec.

### Sanitization passes implemented

1. HTML comment removal (`<!-- … -->`)
2. Dangerous element dropping (open tag + content + close tag)
3. `on*` event handler attribute stripping
4. Explicit `drop_attributes` field stripping (always includes `srcdoc`, `formaction`)
5. `href` / `src` URL scheme sanitization
6. CSS `expression()` and dangerous `url()` style attribute stripping

### Test coverage

- 54 tests, 0 failures
- XSS vectors: script injection, event handlers, javascript: URLs, CSS expressions,
  HTML comment attacks, control character URL bypasses
- `HTML_PASSTHROUGH` verified to pass all content unchanged
- `HTML_RELAXED` verified to allow HTML subset while still stripping scripts

### Design notes

- `HTML_PASSTHROUGH` uses `drop_attributes = false` (not `{}`) to disable all
  attribute stripping. An empty table is truthy in Lua and would still trigger
  on* handler removal.
- Pattern-based: uses Lua `string.gsub` and `string.find`. No external parser
  dependencies. Portable to any Lua 5.4+ environment.
