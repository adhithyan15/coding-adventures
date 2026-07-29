# Changelog

## 0.2.1

- Added a cross-platform Cairo acceptance test proving canned text-only HTML
  traverses parsing, shared layout, paint-scene emission, and rasterization
  into a non-uniform RGBA pixel buffer.
- Kept URI image decoding outside the raster test so resource loading remains
  an explicit follow-up gate rather than a hidden backend responsibility.

## 0.2.0

- Added `LinkRegion` values in logical document-content coordinates.
- Added iterative `extract_link_regions` traversal with absolute coordinate
  accumulation and invalid/empty box filtering.
- Added `hit_test_link` for viewport coordinates plus vertical scroll offset,
  with half-open right and bottom edges.
- Added extracted link regions to `HtmlPaintOutput`.

## 0.1.0

- Added `html_render_tree_to_paint`, the browser-facing composition of
  `html-to-layout`, `layout-block`, and `layout-to-paint`.
- Added `HtmlPaintViewport` and `HtmlPaintOutput`; the output retains both
  positioned geometry and the backend-neutral paint scene.
- Added canned HTML acceptance coverage for background, text, inline link
  styling, resolved image metadata, and scrollable scene height.
