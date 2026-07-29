# html-to-paint

The browser-facing composition seam from an `html-parser`
`BrowserRenderTree` to shared layout geometry and a backend-neutral
`PaintScene`.

```text
BrowserRenderTree
  -> html-to-layout
  -> layout-block
  -> layout-to-paint
  -> HtmlPaintOutput { positioned, links, scene }
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

The returned positioned tree retains preserved `html` metadata, while `links`
contains resolved link rectangles in logical document-content coordinates.
`hit_test_link` converts viewport coordinates using the current vertical scroll
offset. The scene height is at least the viewport height and expands to the
laid-out document height for scrolling.

```rust
let target = hit_test_link(&output.links, mouse_x, mouse_y, scroll_y)
    .map(|region| region.url.as_str());
```

## Current boundary

- HTML source parsing remains in `html-parser`.
- Resource fetching and image decoding are not performed here.
- Host navigation and visited-link policy remain follow-up work.
- Paint backends consume the returned `PaintScene`; Cairo acceptance proves
  text-only HTML reaches RGBA pixels, while host backend selection remains
  outside this package.

Five tests cover viewport normalization, end-to-end canned HTML paint output,
absolute link-region extraction, empty-box filtering, scroll-aware hit testing,
half-open boundary behavior, and real Cairo rasterization into RGBA pixels.
