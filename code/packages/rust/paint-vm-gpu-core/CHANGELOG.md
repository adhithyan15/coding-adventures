# Changelog

## Unreleased

- Added `GpuPaintPlan`, `GpuCommand`, mesh, image-upload, text, and glyph-run
  plan types.
- Added PaintScene lowering for rects, lines, ellipses, flattened paths, clips,
  groups, layers, images, text, and glyph runs.
- Added diagnostics for degraded GPU-core gaps such as path arcs, gradients,
  filters, blend modes, dashed strokes, and exact fill rules.
- Added linear gradient ramp textures with linear sampling metadata for GPU
  backends that support texture sampling.
- Added radial gradient 2D textures with radial UV lowering for GPU backends
  that support sampled gradient textures.
