# layout-ir

Rust implementation of **UI02** — the universal intermediate representation
for layout. Pure types + builders + the `TextMeasurer` trait. Zero
runtime dependencies.

Every downstream layout algorithm (block / flexbox / grid / tex) and
every layout-to-paint converter depends on this crate.

Spec: [code/specs/UI02-layout-ir.md](../../../specs/UI02-layout-ir.md).

## Exports

- **Geometry types:** `SizeValue`, `Edges`, `Constraints`
- **Visual types:** `Color`, `FontSpec`, `TextAlign`, `ImageFit`
- **Content payloads:** `TextContent`, `ImageContent`, `Content`
- **Tree types:** `LayoutNode`, `PositionedNode`
- **Extension bag:** `Ext` / `ExtValue` (zero-dep typed map)
- **TextMeasurer trait + MeasureResult** — the shared measurement contract
- **Builder helpers** — `edges_all`, `rgb`, `font_spec`, `size_fixed`, `constraints_fixed`, `LayoutNode::leaf_text`, etc.

## Design principles (from UI02)

- **Producer-ignorant.** A LayoutNode tree from Markdown looks identical
  to one from Mosaic; algorithms only see the IR.
- **Algorithm-ignorant.** Algorithm-specific data lives in the `ext` bag,
  namespaced by algorithm name.
- **No smartness.** The IR doesn't validate; it's a dumb data structure.
