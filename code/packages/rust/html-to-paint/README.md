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
  -> scene_image_resource_uris
  -> resolve_scene_image_resources_incrementally(browser_resolver)
  -> PaintScene with pending, decoded, or failed images
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

`html_render_tree_to_paint_with_link_state` accepts the same inputs plus a
resolved-URL visited callback. Link color and underline decoration flow through
Layout IR into backend-neutral glyph and rectangle paint instructions.

The returned positioned tree retains preserved `html` metadata, while `links`
contains resolved link rectangles in logical document-content coordinates.
`hit_test_link` converts viewport coordinates using the current vertical scroll
offset. The scene height is at least the viewport height and expands to the
laid-out document height for scrolling.

```rust
let target = hit_test_link(&output.links, mouse_x, mouse_y, scroll_y)
    .map(|region| region.url.as_str());
```

`resolve_scene_image_resources` accepts a host-owned `HtmlImageFetcher`.
The host chooses HTTP, file, cache, and security policy; this package decodes
fetched GIF or baseline JPEG bytes into `ImageSrc::Pixels`. Resolution is
atomic: the input scene is unchanged when any resource fails.

Browser hosts can instead call
`resolve_scene_image_resources_with_mosaic_fallback`. That tolerant path keeps
successful images, replaces each failure with a clipped black border and its
HTML `alt` text, and returns the failures for host diagnostics.

Incremental browsers use `scene_image_resource_uris` for stable first-paint
request order and `resolve_scene_image_resources_incrementally` with the
tri-state `HtmlImageResource` contract. Pending images preserve geometry with
an alt-text placeholder but do not produce failures; ready bytes decode to
pixels, and failed/decode states reuse the normal recoverable fallback.

## Current boundary

- HTML source parsing remains in `html-parser`.
- Resource transport remains host-owned behind `HtmlImageFetcher`.
- GIF and baseline JPEG decoding convert fetched bytes into shared pixels.
- Mosaic broken-image borders and alt text recover fetch/decode failures.
- Host navigation remains outside this package; visited membership is supplied
  as a narrow callback rather than imported browser state.
- Paint backends consume the returned `PaintScene`; Cairo acceptance proves
  both text-only and inline-image HTML reaches RGBA pixels, while host backend
  selection remains outside this package.

Tests cover viewport normalization, end-to-end canned HTML paint output,
absolute link-region extraction, empty-box filtering, scroll-aware hit testing,
half-open boundary behavior, visited/unvisited glyph and underline paint,
atomic image resolution, GIF/JPEG decoding, and real Cairo rasterization of
decoded and broken-image fallback pixels.
