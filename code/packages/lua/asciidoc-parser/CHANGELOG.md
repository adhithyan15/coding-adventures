# Changelog — coding-adventures-asciidoc-parser

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial release.
- Block parser state machine: normal, paragraph, code_block, literal_block,
  passthrough_block, quote_block, unordered_list, ordered_list states.
- AsciiDoc headings (`= H1` through `====== H6`).
- Thematic breaks (`'''` three or more single-quotes).
- Fenced code blocks (`----`), literal blocks (`....`),
  passthrough blocks (`++++`), quote blocks (`____`).
- `[source,lang]` attribute blocks to set language on the next code block.
- Unordered lists (`* item`, `** nested`) and ordered lists (`. item`, `.. nested`).
- Comments (`// text`) are silently skipped.
- Inline parser: strong (`*bold*`), emphasis (`_italic_`),
  unconstrained variants (`**bold**`, `__italic__`), inline code (`` `code` ``),
  links (`link:url[text]`), images (`image:url[alt]`),
  cross-references (`<<anchor,text>>`), bare URLs (`https://...`).
- Produces Document AST node tables compatible with
  `coding_adventures.document_ast_to_html`.
