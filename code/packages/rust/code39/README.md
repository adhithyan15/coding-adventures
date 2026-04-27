# code39

Dependency-free Code 39 encoder that emits shared barcode runs and paint-ready
layout output.

This crate now follows the paint pipeline:

```text
input
  -> normalize + encode
  -> expand into Code 39 barcode runs
  -> barcode-layout-1d
  -> PaintScene
  -> paint VM backend
  -> PixelContainer
  -> PNG / other codec
```

The `layout_code39()` convenience function stops at `PaintScene` so Metal,
Direct2D, GDI, and future native extension bridges can all consume the same
output.
