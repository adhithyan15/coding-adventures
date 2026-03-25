# Changelog — coding-adventures-document-ast-to-html (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-03-24

### Added

- Initial release of the Lua Document AST to HTML renderer.
- `to_html(node, opts)` — renders any Document AST node to an HTML string.
- Full rendering support for all CommonMark 0.31.2 block node types:
  `document`, `heading` (levels 1–6), `paragraph`, `code_block` (with optional
  `language-X` class), `blockquote`, `list` (ordered and unordered, with
  optional `start` attribute), `list_item`, `thematic_break`, `raw_block`.
- Full rendering support for all CommonMark 0.31.2 inline node types:
  `text`, `emphasis`, `strong`, `code_span`, `link`, `image`, `autolink`
  (URL and email), `raw_inline`, `hard_break`, `soft_break`.
- **Tight list rendering** — paragraphs inside tight list items are rendered
  without `<p>` wrappers; loose list items retain `<p>` wrappers.
- **HTML escaping** — all text literals escape `&`, `<`, `>`, and `"`.
- **URL sanitization** — `javascript:`, `vbscript:`, and `data:` schemes in
  `href`/`src` attributes are replaced with `""`, unconditionally.
- **`sanitize` option** — when `opts.sanitize = true`, raw HTML blocks and raw
  HTML inlines are suppressed (empty string) rather than passed through.
- 52-test suite covering every node type, HTML escaping, URL sanitization,
  tight/loose list variants, and the sanitize option; all tests pass with busted.
- LuaRocks rockspec (`coding-adventures-document-ast-to-html-0.1.0-1.rockspec`).
- BUILD and BUILD_windows files for the monorepo build tool.
