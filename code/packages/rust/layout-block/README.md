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
- Nested containers with padding on both outer and inner sides.
- Text leaves — width/height resolved by the supplied measurer with
  wrap-at-max-width semantics.
- Image leaves — sized from node's `width`/`height` hints.
- Size hints: `Fill` / `Wrap` / `Fixed(v)` with `min_width` / `max_width`
  / `min_height` / `max_height` clamping.

Out of scope for v1 (per UI07):
- Mixed block + inline children with anonymous-block promotion.
- `float` / `clear` / absolute positioning.
- RTL / bidirectional text.
- CSS columns.
- Parent-child margin collapse.

The upstream `document-ast-to-layout` converter produces strictly
block-or-leaf trees, so the missing inline-flow path is not a blocker
for Markdown rendering.

## Tests

14 unit tests cover single leaves, block stacking, margin collapsing,
nested containers, size hints, min/max clamping, content passthrough,
empty containers, and a realistic document shape.
