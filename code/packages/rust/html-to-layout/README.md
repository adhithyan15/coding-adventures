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
- `html_render_tree_to_layout_with_link_state(..., is_visited) -> LayoutNode`
- `mosaic_html_theme() -> HtmlTheme`

The adapter maps parser-supplied display categories, text, headings, images,
hidden content, stable IDs, and resolved link/resource URLs. Browser metadata
needed after layout is retained under `LayoutNode.ext["html"]`, so positioned
nodes still carry link targets and semantic roles for hit testing.
The optional visited callback receives only resolved URLs. It selects theme
colors and inherited link decoration without exposing history or persistence
policy to this producer adapter.

This bridge intentionally does not implement CSS cascade or inline layout.
It projects display, preformatted whitespace, and producer metadata into the
shared IR; `layout-block` and `layout-inline` own word fragmentation, baseline
alignment, and semantic wrapper geometry independently of HTML.

## Verification

From `code/packages/rust`:

```sh
cargo test -p html-to-layout
cargo clippy -p html-to-layout --all-targets -- -D warnings
```
