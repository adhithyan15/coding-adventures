# Changelog

## [0.1.0] — initial release

### Added
- `layout_block(&LayoutNode, Constraints, &impl TextMeasurer) -> PositionedNode` — the UI07 block layout entry point.
- Block container layout: stacks children vertically, applies padding, computes parent height from accumulated children.
- Text leaf layout: delegates to `TextMeasurer` with max-width for wrapping; resolves `Wrap` / `Fill` / `Fixed` width hints; clamps by `min_width`/`max_width`.
- Image leaf layout: sizes from explicit `width`/`height` hints (intrinsic-size resolution is a v2 concern).
- Sibling margin collapsing per CSS rule: adjacent block siblings' `margin.bottom` + `margin.top` collapse to the max of the two.
- Size resolution respects `min_width` / `max_width` / `min_height` / `max_height` clamping.
- Nested container recursion preserves parent padding + child margin offsets correctly.

### Tests — 14 pass
- Single text leaf sized by measurer, wrap vs fill vs fixed widths, multi-line text wrap.
- Two-block stacking with correct vertical offsets.
- Padded container offsets children by padding amount and includes padding in own height.
- Margin collapsing between sibling blocks.
- Nested containers propagate padding correctly through two levels.
- Fixed height override, fill width from constraints, max-width clamping.
- Content and id passthrough from input to output.
- Empty containers: zero height; empty with padding: padding height.
- End-to-end realistic document (h1 + paragraph with margins).

### Design
- Zero external dependencies beyond `layout-ir`.
- Generic over any `TextMeasurer` impl — layout and text measurement are fully decoupled, matching the spec.
- v1 explicit non-goals (inline flow mixed with block, float / clear, RTL, CSS columns, parent-child margin collapse) match the UI07 spec's documented deferrals.
