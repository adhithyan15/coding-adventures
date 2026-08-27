# layout-block

UI07 — block-and-inline flow layout in Rust. Takes a `LayoutNode` tree
+ `Constraints` + `TextMeasurer` and returns a `PositionedNode` tree.

Spec: [code/specs/UI07-layout-block.md](../../../specs/UI07-layout-block.md).

## Exports

- `layout_block(&LayoutNode, Constraints, &impl TextMeasurer) -> PositionedNode`

## v1 scope

Handles:
- Block containers — stack children vertically with margin collapsing
  between adjacent siblings.
- Inline flow delegates to `layout-inline` for word fragmentation, semantic
  wrapper splitting, measured baseline alignment, and explicit line breaks.
- Nested containers with padding on both outer and inner sides.
- Text leaves — width/height resolved by the supplied measurer with
  wrap-at-max-width semantics.
- Image leaves — sized from node's `width`/`height` hints.
- Size hints: `Fill` / `Wrap` / `Fixed(v)` with `min_width` / `max_width`
  / `min_height` / `max_height` clamping.

Out of scope for v1 (per UI07):
- Unicode UAX #14 line-breaking and bidirectional text.
- Full CSS inline edge-decoration continuation across fragments.
- `float` / `clear` / absolute positioning.
- RTL / bidirectional text.
- CSS columns.
- Parent-child margin collapse.

Nodes without `ext["block"]["display"]` remain block-level by default, so
existing `document-ast-to-layout` output preserves its original geometry.
`html-to-layout` supplies explicit display metadata for browser content.

## Tests

18 unit tests cover single leaves, block stacking, atomic inline placement and
wrapping, margin collapsing,
nested containers, size hints, min/max clamping, content passthrough,
empty containers, and a realistic document shape.
