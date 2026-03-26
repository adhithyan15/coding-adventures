# Changelog — coding-adventures-commonmark-parser (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-03-24

### Added

- Initial release of the Lua CommonMark 0.31.2 parser.
- Two-phase parsing: block structure pass followed by inline content pass.
- **Block nodes produced**: `document`, `heading` (ATX and setext), `paragraph`,
  `code_block` (indented and fenced), `blockquote`, `list` (ordered/unordered,
  tight/loose), `list_item`, `thematic_break`, `raw_block` (HTML blocks types
  1–7), link reference definitions (consumed, not emitted as nodes).
- **Inline nodes produced**: `text`, `emphasis`, `strong`, `code_span`, `link`
  (inline and reference forms), `image`, `autolink`, `raw_inline`, `hard_break`,
  `soft_break`.
- Full CommonMark delimiter run algorithm for emphasis/strong including
  UTF-8 aware left-/right-flanking detection.
- Unicode case folding in link label normalisation covering Latin-1, Latin
  Extended-A/B, Greek, Cyrillic, and Armenian code points.
- Entity reference decoding: named (`&amp;`), decimal (`&#38;`), and hex
  (`&#x26;`) forms; digit-count limits (≤7 decimal digits, ≤6 hex digits)
  enforced per spec.
- HTML block type 7 detection uses a proper attribute scanner (attribute names
  must be preceded by whitespace) so URLs like `<https://…>` are not misclassified.
- Tab expansion in list marker parsing: a single tab after a marker counts as
  one space (matching the spec's `( +|\t|$)` rule).
- List tightness: blank line within a multi-child item propagates `loose` to
  the parent list; blank line after the last item does not.
- Link title parsing: titles spanning a blank line are rejected.
- HTML block output: trailing blank lines are stripped from type 6/7 block
  content (the terminating blank line is not part of the block).
- `scanner.lua` — line/column tracking scanner used by the block parser.
- `entities.lua` — entity decoding utilities.
- `entity_table.lua` — full HTML5 named entity table (2,231 entries); corrected
  `&quot;` mapping from `\` to `"`.
- 652/652 CommonMark 0.31.2 specification examples pass.
- 77-test unit suite (`test_commonmark_parser.lua`) covering all block and
  inline node types; all tests pass with busted.
- LuaRocks rockspec (`coding-adventures-commonmark-parser-0.1.0-1.rockspec`).
- BUILD and BUILD_windows files for the monorepo build tool.
