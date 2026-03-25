# Changelog — coding-adventures-document-ast-to-html

## [0.1.0] — 2026-03-24

### Added

- Initial Python port of the TypeScript `@coding-adventures/document-ast-to-html` package.
- `to_html(document, options?)` function that renders a DocumentNode to an HTML string.
- `RenderOptions(sanitize=True)` for stripping raw HTML from untrusted input.
- Full node type coverage: all block and inline Document AST node types.
- URL sanitization to block `javascript:`, `vbscript:`, `data:`, `blob:` schemes.
- HTML escaping via `escape_html()` for XSS protection in text content and attributes.
- 93% test coverage.
