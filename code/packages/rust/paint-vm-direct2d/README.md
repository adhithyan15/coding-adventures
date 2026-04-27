# paint-vm-direct2d

Direct2D GPU renderer for the `paint-instructions` scene model.

## Overview

`paint-vm-direct2d` takes a `PaintScene` and renders it to a `PixelContainer`
using Microsoft's Direct2D API — a GPU-accelerated 2D rendering API available
on Windows Vista and later.

This is the modern, hardware-accelerated renderer in the paint-* stack on
Windows. For a simpler CPU-based fallback, see `paint-vm-gdi`.

## Where It Fits

```
Producer (barcode, chart, layout engine)
  → PaintScene                          (paint-instructions)
  → paint-vm-direct2d                   (THIS CRATE)
  → PixelContainer                      (pixel-container)
  → paint-codec-png / paint-codec-webp  (image codec)
  → PNG / WebP bytes
```

## Usage

```rust
use paint_instructions::{PaintScene, PaintInstruction, PaintRect};
use paint_vm_direct2d::render;

let mut scene = PaintScene::new(200.0, 100.0);
scene.instructions.push(PaintInstruction::Rect(
    PaintRect::filled(10.0, 10.0, 180.0, 80.0, "#ff0000"),
));

let pixels = render(&scene);
assert_eq!(pixels.width, 200);
assert_eq!(pixels.height, 100);
```

## Supported Instructions

| Instruction   | Status |
|---------------|--------|
| PaintRect     | Fully implemented |
| PaintLine     | Fully implemented |
| PaintGroup    | Fully implemented |
| PaintClip     | Fully implemented |
| PaintGlyphRun | Planned |
| PaintEllipse  | Planned |
| PaintPath     | Planned |
| PaintLayer    | Planned |
| PaintGradient | Planned |
| PaintImage    | Planned |

## Platform

Windows Vista or later. Direct2D uses the GPU if available, falling back to
the WARP software rasteriser. On macOS use `paint-metal`, on Linux use
`paint-vm-cairo` (future).

## Technical Details

- Uses COM (Component Object Model) via the `windows` crate
- Renders to an offscreen WIC bitmap (no window/HWND needed)
- Pixels are in premultiplied BGRA format internally, converted to straight
  RGBA on readback
- Supports antialiasing (per-primitive mode by default)
