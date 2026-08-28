# Changelog

## Unreleased

- Added `GpuImageResolver` and `plan_scene_with_image_resolver` so host-owned
  resource pipelines can lower URI-backed images without moving fetch or codec
  policy into a GPU backend.
- Added `GpuPaintPlan`, `GpuCommand`, mesh, image-upload, text, and glyph-run
  plan types.
- Added PaintScene lowering for rects, lines, ellipses, flattened paths, clips,
  groups, layers, images, text, and glyph runs.
- Added diagnostics for degraded GPU-core gaps such as path arcs, gradients,
  filters, blend modes, and exact fill rules.
- Added linear gradient ramp textures with linear sampling metadata for GPU
  backends that support texture sampling.
- Added radial gradient 2D textures with radial UV lowering for GPU backends
  that support sampled gradient textures.
- Added a shared Tier 1 textured backend profile for GPU/compute backends that
  accept image, linear-gradient, and radial-gradient texture plans.
- Added dashed `PaintLine` lowering to segment meshes instead of degrading line
  dashes to solid strokes.
- Added dashed `PaintPath` stroke lowering that carries dash state across
  flattened path segments.
- Added dashed `PaintRect` stroke lowering around the rectangle perimeter.
- Added dashed `PaintEllipse` stroke lowering around the sampled ellipse
  perimeter.
- Added square and round `PaintLine` cap lowering for GPU mesh plans.
- Added square and round open-contour `PaintPath` cap lowering, including caps
  on dashed path segments.
- Added solid `PaintPath` stroke join lowering for bevel, round, and bounded
  miter joins.
- Added dashed `PaintPath` continuing-run join lowering so dash runs that cross
  contour corners share the same bevel, round, and bounded miter join geometry.
