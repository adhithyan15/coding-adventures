# html-to-layout

`html-to-layout` is the browser-facing front end for the shared Rust layout
stack. It converts the `BrowserRenderTree` produced by `html-parser` into a
`layout-ir::LayoutNode` tree:

```text
HTML source
  -> html-parser
  -> BrowserRenderTree
  -> html-to-layout
  -> LayoutNode
  -> layout-block
  -> PositionedNode
  -> layout-to-paint
```

## API

- `html_render_tree_to_layout(&BrowserRenderTree, &HtmlTheme) -> LayoutNode`
- `mosaic_html_theme() -> HtmlTheme`

The adapter maps parser-supplied display categories, text, headings, images,
hidden content, stable IDs, and resolved link/resource URLs. Browser metadata
needed after layout is retained under `LayoutNode.ext["html"]`, so positioned
nodes still carry link targets and semantic roles for hit testing.

This first bridge intentionally does not implement CSS cascade or a complete
CSS inline-formatting context. Inline nodes remain explicit layout containers;
the current `layout-block` engine positions them using its documented v1 block
flow. A dedicated inline layout pass is the next rendering-fidelity boundary,
not parser conformance work.

## Verification

From `code/packages/rust`:

```sh
cargo test -p html-to-layout
cargo clippy -p html-to-layout --all-targets -- -D warnings
```
