# Changelog — @coding-adventures/layout-block

## Unreleased

### Fixed

- Inline tokens (words sliced out of a text node during word-wrap) no longer carry the parent node's full text value. Previously each token's `content.value` was the entire string, which caused paint backends to render the whole paragraph at every word position. The token now carries just its own word — correct for PaintText backends (which would double-paint severely) and neater for PaintGlyphRun backends (which used to rely on per-glyph positioning to accidentally mask the bug).
- Trailing whitespace on a text leaf (e.g. `"is "` in `"is **bold**"`) now produces `spaceAfter` on the last emitted token, so adjacent inline leaves are separated by a space instead of colliding into `"isbold"`.
- Pure-whitespace text leaves (e.g. a `soft_break` with `value = " "`) now emit a single zero-width spacer token with `spaceAfter = spaceWidth`. Previously such leaves produced no tokens at all, silently dropping the space between words on either side (`"noDOMelements"`).

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of `layout_block(container, constraints, measurer) → PositionedNode`
- `BlockExt` interface for `ext["block"]` metadata (display, overflow, whiteSpace, wordBreak, verticalAlign, paragraphSpacing)
- Block formatting context: block children stack vertically, fill available width
- Inline formatting context: inline children flow horizontally and wrap to new lines
- Anonymous block generation for mixed block/inline sibling lists (CSS anonymous box rule)
- CSS margin collapsing for adjacent sibling block children (sibling-only, not parent-child)
- Word-level text wrapping for `whiteSpace: "normal"` (default)
- `whiteSpace: "nowrap"` and `whiteSpace: "pre"` — prevent wrapping
- Vertical alignment within line boxes: `baseline` / `top` / `middle` / `bottom`
- Padding support on block containers
- `overflow: "hidden"` / `"scroll"` propagation to `ext["paint"]`
- Leaf text/image node measurement
- `paragraphSpacing` for extra vertical gap between paragraphs
- Comprehensive tests with >90% coverage
