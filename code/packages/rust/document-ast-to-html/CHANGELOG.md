# Changelog — document-ast-to-html

## 0.1.0 — 2026-03-24

Initial release.

### Added

- `to_html(&DocumentNode, &RenderOptions) -> String` — render a Document AST to HTML
- `RenderOptions` with `sanitize: bool` field for stripping raw HTML
- All block node renderers: heading, paragraph, code block, blockquote, list, list item, thematic break, raw block
- All inline node renderers: text, emphasis, strong, code span, link, image, autolink, raw inline, hard break, soft break
- Tight vs loose list detection (suppresses `<p>` tags in tight list items)
- `escape_html(text: &str) -> String` — HTML-escape `&`, `<`, `>`, `"`
- `sanitize_url(url: &str) -> String` — strip control chars and block dangerous schemes (`javascript:`, `vbscript:`, `data:`, `blob:`)
- `normalize_url_for_attr(url: &str) -> String` — percent-encode characters not safe in HTML href/src attributes
- Unit tests and doctests
