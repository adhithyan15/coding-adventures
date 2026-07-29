# html-to-paint

The browser-facing composition seam from an `html-parser`
`BrowserRenderTree` to shared layout geometry and a backend-neutral
`PaintScene`.

```text
BrowserRenderTree
  -> html-to-layout
  -> layout-block
  -> layout-to-paint
  -> HtmlPaintOutput { positioned, scene }
```

The package keeps the individual parser, adapter, layout, and paint stages
replaceable while giving Venture and other browser hosts one executable
pipeline entry point. Callers supply the text measurer and the matching
TXT00 resolver/metrics/shaper trio, so platform font policy stays outside
this package.

## API

```rust
pub fn html_render_tree_to_paint<M, S, FM, R>(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
    viewport: HtmlPaintViewport,
    measurer: &M,
    shaper: &S,
    metrics: &FM,
    resolver: &R,
) -> HtmlPaintOutput
```

The returned positioned tree is retained because the next browser slice uses
its preserved `html` metadata to derive link hit regions. The scene height is
at least the viewport height and expands to the laid-out document height for
scrolling.

## Current boundary

- HTML source parsing remains in `html-parser`.
- Resource fetching and image decoding are not performed here.
- Link hit-region extraction and host navigation remain follow-up work.
- Paint backends consume the returned `PaintScene`; this package does not
  rasterize it.

Two tests cover viewport normalization and the end-to-end canned HTML
acceptance path, including page background, heading text, inline link color,
resolved image URI, and document-height expansion.
