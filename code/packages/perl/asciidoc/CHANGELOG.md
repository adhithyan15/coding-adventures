# Changelog — CodingAdventures::Asciidoc

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial release.
- `to_html($text)` — parse AsciiDoc and return HTML string.
- `parse($text)`   — parse AsciiDoc and return arrayref of block hashrefs.
- Block parser state machine: normal, paragraph, code_block, literal_block,
  passthrough_block, quote_block, unordered_list, ordered_list.
- AsciiDoc headings (`= H1` through `====== H6`).
- Thematic breaks (`'''` three or more single quotes).
- Code blocks (`----`), literal blocks (`....`),
  passthrough blocks (`++++`), quote blocks (`____`).
- `[source,lang]` attribute blocks set language on next code block.
- Unordered lists (`* item`, `** nested`) and ordered lists (`. item`).
- Comment lines (`// text`) are silently skipped.
- Inline: strong (`*text*`, `**text**`), emphasis (`_text_`, `__text__`),
  inline code (`` `code` ``), link (`link:url[text]`),
  image (`image:url[alt]`), cross-reference (`<<anchor,text>>`),
  bare URLs (`https://...`), hard/soft breaks.
- HTML escaping for `&`, `<`, `>`, `"` in all text content.
- Full self-contained implementation — no external Perl dependencies.
