# Changelog — @coding-adventures/layout-block

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
