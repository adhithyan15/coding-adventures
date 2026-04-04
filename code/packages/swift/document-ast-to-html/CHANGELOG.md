# Changelog — document-ast-to-html (Swift)

All notable changes to this package are documented here.

## 0.1.0 — Initial release

- `render(_ node: BlockNode) -> String` — renders any Document AST block node to HTML
- `htmlEscape(_ s: String) -> String` — public HTML escaping utility
- Full rendering support for all block types: document, heading, paragraph, code block, blockquote, ordered and unordered lists, task items, thematic break, raw block (HTML), table with thead/tbody and alignment
- Full rendering support for all inline types: text, emphasis, strong, strikethrough, code span, link (with title), image (with title), autolink (URL and email), raw inline (HTML), hard break, soft break
- Tight list support: single-paragraph items rendered without `<p>` wrappers
- URL safety: `&` escaped in all `href`/`src` attributes; autolink URLs percent-encoded
- Comprehensive test suite with 50+ test cases
