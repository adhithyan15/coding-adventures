# paint-vm-skia

Skia backend for the `paint-instructions` scene model.

## Overview

`paint-vm-skia` renders a `PaintScene` into an offscreen Skia raster surface and
returns a `PixelContainer`. The first implementation is deliberately CPU-raster
first so it can run in headless CI and on developer machines before the GPU
surface paths are added.

This backend is meant to become the high-quality portable Paint VM path across
Windows, macOS, Linux, and BSD. Skia gives us a mature vector rasterizer, image
pipeline, gradients, text APIs, and a later path to GPU acceleration.

## Where It Fits

```text
Producer (barcode, diagram, layout engine)
  -> PaintScene                         (paint-instructions)
  -> paint-vm-skia                      (THIS CRATE)
  -> PixelContainer                     (pixel-container)
  -> paint-codec-png / paint-codec-webp (image codec)
  -> PNG / WebP bytes
```

## Usage

```rust
use paint_instructions::{PaintInstruction, PaintRect, PaintScene};
use paint_vm_skia::render;

let mut scene = PaintScene::new(200.0, 100.0);
scene.instructions.push(PaintInstruction::Rect(
    PaintRect::filled(10.0, 10.0, 180.0, 80.0, "#ff0000"),
));

let pixels = render(&scene).unwrap();
assert_eq!((pixels.width, pixels.height), (200, 100));
```

## Supported Instructions

| Instruction | Status |
|-------------|--------|
| `PaintRect` | Implemented, including rounded rects |
| `PaintLine` | Implemented, including caps and dashes |
| `PaintEllipse` | Implemented |
| `PaintPath` | Implemented for move/line/quad/cubic/close; `ArcTo` deferred |
| `PaintText` | Degraded simple text via Skia font APIs |
| `PaintGlyphRun` | Degraded positioned glyph drawing via Skia font APIs |
| `PaintClip` | Implemented for rectangular clips |
| `PaintGroup` | Implemented with transform and opacity |
| `PaintLayer` | Implemented for transform and opacity; filters/blends deferred |
| `PaintGradient` | Implemented for linear/radial fills referenced by `url(#id)` |
| `PaintImage` | Implemented for `ImageSrc::Pixels` |

## Notes

- The renderer reads back `RGBA8888` unpremultiplied pixels from Skia.
- Text is intentionally marked degraded until we wire `SkShaper`, `SkParagraph`,
  HarfBuzz/Pango-style shaping, fallback, and exact `font_ref` semantics.
- GPU surfaces are not required for this Tier 1 path. Future work can add
  Ganesh/Graphite surfaces while keeping the same `PaintRenderer` contract.
