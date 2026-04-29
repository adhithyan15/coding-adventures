# Changelog

## Unreleased

- Added `GpuPaintPlan`, `GpuCommand`, mesh, image-upload, text, and glyph-run
  plan types.
- Added PaintScene lowering for rects, lines, ellipses, flattened paths, clips,
  groups, layers, images, text, and glyph runs.
- Added diagnostics for degraded GPU-core gaps such as path arcs, gradients,
  filters, blend modes, dashed strokes, and exact fill rules.
