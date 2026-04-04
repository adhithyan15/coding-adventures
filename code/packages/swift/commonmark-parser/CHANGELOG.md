# Changelog — commonmark-parser (Swift)

All notable changes to this package are documented here.

## 0.1.0 — Initial release

- `parse(_ markdown: String) -> BlockNode` — public entry point, returns `.document(...)` node
- `BlockParser` — Phase 1 line-by-line structural parser:
  - ATX headings (h1–h6) with trailing hash stripping
  - Thematic breaks (`---`, `***`, `___`, 3+ chars)
  - Fenced code blocks (backtick and tilde fences, with info string)
  - Indented code blocks (4-space indent)
  - Blockquotes (`> `) with recursive parsing and nested support
  - Unordered lists (`-`, `*`, `+`) with tight/loose detection
  - Ordered lists (`.` and `)` markers) with start number support
  - Raw HTML blocks (lines starting with `<`)
  - Paragraphs (blank-line separated)
- `InlineParser` — Phase 2 inline content parser:
  - Emphasis (`*text*`, `_text_`) and strong (`**text**`, `__text__`)
  - Code spans (single and double backtick)
  - Links `[text](url "title")` with title support
  - Images `![alt](url "title")` with title support
  - Autolinks: URL (`<https://...>`) and email (`<user@email>`)
  - Hard break (two trailing spaces) and soft break (newline)
  - Strikethrough GFM extension (`~~text~~`)
  - Backslash escapes for ASCII punctuation
- 50+ test cases covering all supported features
