# Changelog

## 0.1.0

- Added `html_render_tree_to_paint`, the browser-facing composition of
  `html-to-layout`, `layout-block`, and `layout-to-paint`.
- Added `HtmlPaintViewport` and `HtmlPaintOutput`; the output retains both
  positioned geometry and the backend-neutral paint scene.
- Added canned HTML acceptance coverage for background, text, inline link
  styling, resolved image metadata, and scrollable scene height.
