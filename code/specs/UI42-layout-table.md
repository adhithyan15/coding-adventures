# UI42: Table Formatting Context

## Status

Implemented reusable browser profile.

## Contract

Producers encode computed values in `LayoutNode.ext["table"]`:

- `layout`: `auto` or `fixed`
- `borderCollapse`: `separate` or `collapse`
- `borderSpacingX`, `borderSpacingY`: finite non-negative logical lengths
- `captionSide`: `top` or `bottom`
- `columnSpan`, `rowSpan`: positive integers
- `verticalAlign`: `top`, `middle`, or `bottom`
- `sectionKind`: optional `thead`, `tbody`, or `tfoot` metadata

`layout-table` owns tolerant decoding, diagnostics, anonymous row/cell
normalization, section ordering, slot occupancy, column and row sizing, span
distribution, caption placement, and cell alignment. It delegates caption and
cell subtree layout through the same callback used by block, flex, and grid,
keeping text, paint, clipping, and host integration outside the algorithm.

Automatic layout computes minimum and preferred widths from reusable text
measurement and distributes definite free space between those bounds. Fixed
layout honors column and first-row width hints before sharing remaining space.
Separate borders reserve horizontal/vertical spacing; collapsed borders use a
zero-spacing geometry. Minimum-content widths may intentionally exceed a
definite table width so the existing overflow contract can clip or scroll.

HTML table section roles, `<col span>`, `colspan`, and `rowspan` are mapped by
`html-to-layout`; CSS syntax remains owned by `css-parser`. The positioned tree
therefore reaches `layout-to-paint`, link geometry, visual fixtures, and every
native/generated Venture host without toolkit-specific table behavior.

## Bounded follow-ups

- conflict resolution for collapsed border colors, styles, and widths
- baseline alignment and writing modes
- percentage row heights and paged fragmentation
- dynamic column invalidation for incremental DOM mutations
