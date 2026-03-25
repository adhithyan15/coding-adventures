# Changelog — @coding-adventures/document-ast-to-html

## [0.1.0] — 2026-03-24

### Added

- Initial release of the Document AST → HTML renderer.
- `toHtml(doc: DocumentNode): string` — renders any Document AST to HTML.
- Full mapping from all 19 Document AST node types to HTML:
  - Block nodes: `document`, `heading`, `paragraph`, `code_block`,
    `blockquote`, `list`, `list_item`, `thematic_break`, `raw_block`
  - Inline nodes: `text`, `emphasis`, `strong`, `code_span`, `link`,
    `image`, `autolink`, `raw_inline`, `hard_break`, `soft_break`
- `RawBlockNode` / `RawInlineNode` rendering:
  - `format: "html"` → emit value verbatim
  - `format: other` → skip silently (not an error)
- URL sanitization:
  - Blocks `javascript:`, `vbscript:`, `data:` schemes to prevent XSS
  - C0 control character stripping before scheme detection
  - Safe schemes (irc://, ftp://, mailto://, etc.) pass through unchanged
- Tight vs loose list rendering:
  - Tight lists: `<li>text</li>` (no `<p>` wrappers)
  - Loose lists: `<li><p>text</p></li>`
- Full test suite covering every node type and edge cases.

### Extracted from

Extracted from `@coding-adventures/commonmark` as part of the Document AST
package split (spec TE00). The renderer logic is identical to what was in
`commonmark/src/html-renderer.ts`, adapted to use `RawBlockNode` /
`RawInlineNode` instead of `HtmlBlockNode` / `HtmlInlineNode`.
