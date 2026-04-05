# Changelog — @coding-adventures/layout-flexbox

## 0.1.0 — 2026-04-05

Initial release.

### Added

- `layout_flexbox(container, constraints, measurer)` — main layout function
- `measure_node(node, constraints, measurer)` — natural size helper
- `FlexContainerExt` interface (direction, wrap, alignItems, justifyContent, gap, rowGap, columnGap)
- `FlexItemExt` interface (grow, shrink, basis, alignSelf, order)
- `FlexExt` combined type
- `Size` type `{ width, height }`
- Full 12-step CSS flexbox algorithm: container sizing, main/cross axis, hypothetical sizes, flex lines, grow/shrink distribution, cross-axis sizing, justifyContent, alignItems, padding offset
- Supported justifyContent: start, end, center, between, around, evenly
- Supported alignItems/alignSelf: start, end, center, stretch
- Multi-line wrap (nowrap / wrap)
- order property for sort-before-layout
- min/maxWidth, min/maxHeight clamping on containers and items
