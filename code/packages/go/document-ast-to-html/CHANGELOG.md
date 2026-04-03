# Changelog — document-ast-to-html (Go)

## [0.2.0] — 2026-04-02

### Changed
- Wrapped all public functions with the Operations system (`StartNew[T]`):
  `ToHtml`, `EscapeHtml`.
- Every public call now has automatic timing, structured logging, and panic
  recovery via the capability-cage Operations infrastructure.
- Public API signatures are unchanged.

## [0.1.0] — 2026-03-24

### Added

- Initial Go port of the Document AST → HTML renderer (translated from the
  TypeScript reference implementation, spec TE02).
- `ToHtml(doc *documentast.DocumentNode, opts RenderOptions) string` — top-level
  render function with optional `Sanitize` flag.
- `RenderOptions` struct with `Sanitize bool` field; when true, `javascript:`,
  `vbscript:`, `data:`, and `blob:` URL schemes are blocked in `href`/`src`
  attributes.
- Block node renderers: heading (`<h1>`–`<h6>`), paragraph (`<p>`), code block
  (`<pre><code>`), blockquote, ordered/unordered list, list item, thematic break
  (`<hr />`), raw HTML block (verbatim pass-through or suppressed when
  `Sanitize` is true).
- Tight vs loose list rendering: tight lists suppress the `<p>` wrapper around
  list item content.
- Ordered list `start` attribute emitted whenever the start number is not 1
  (including `start="0"`).
- Inline node renderers: text (HTML-escaped), emphasis, strong, code span,
  link, image, autolink, raw inline HTML, hard break (`<br />\n`), soft break
  (`\n`).
- `EscapeHtml(text string) string` — exported helper that escapes `&`, `<`,
  `>`, `"`, and `'`.
- `sanitizeURL(url string) string` — internal helper that returns `#` for
  blocked URL schemes.
