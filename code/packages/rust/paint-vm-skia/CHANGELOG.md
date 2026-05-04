# Changelog

## Unreleased

- Replaced the scaffold with a real Skia raster Paint VM backend.
- Added rendering for rects, lines, ellipses, cubic/quadratic paths, clips,
  groups, layers, pixel images, simple text, positioned glyph runs, and
  referenced linear/radial gradients.
- Added runtime-selection metadata with Tier 1 capabilities and explicit
  degraded text/glyph support.
- Added unit coverage for primitive rendering, clipping, transforms, text,
  glyph runs, image blitting, gradients, and degraded-text runtime selection.
