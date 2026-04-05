# Changelog — @coding-adventures/layout-grid

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of `layout_grid(container, constraints, measurer) → PositionedNode`
- `GridContainerExt` — templateColumns, templateRows, columnGap, rowGap, autoRows, autoColumns, autoFlow, alignItems, justifyItems
- `GridItemExt` — columnStart, columnEnd, columnSpan, rowStart, rowEnd, rowSpan, alignSelf, justifySelf
- Track list parser: fixed px, flexible fr, auto, minmax(), repeat()
- Explicit item placement (columnStart/rowStart/columnEnd/rowEnd)
- Auto-placement algorithm for `autoFlow: "row"` and `autoFlow: "column"`
- Implicit track generation using autoRows / autoColumns
- Track size resolution: fixed → auto (content-sized) → fr (free space distribution)
- Item alignment within cells: `justifySelf` / `alignSelf` with start/center/end/stretch
- Container `alignItems` / `justifyItems` defaults
- Padding on container
- Fixed container height support
- Comprehensive tests with >96% coverage
