# paint-vm-gdi

GDI (Graphics Device Interface) renderer for the `paint-instructions` scene model.

## Overview

`paint-vm-gdi` takes a `PaintScene` and renders it to a `PixelContainer` using
Windows GDI — the original CPU-based drawing API available since Windows 1.0.

This is the fallback renderer in the paint-* stack. It requires no COM
initialization, no GPU, and works on every version of Windows. The trade-off
is no hardware acceleration and no antialiasing.

## Where It Fits

```
Producer (barcode, chart, layout engine)
  → PaintScene                          (paint-instructions)
  → paint-vm-gdi                        (THIS CRATE)
  → PixelContainer                      (pixel-container)
  → paint-codec-png / paint-codec-webp  (image codec)
  → PNG / WebP bytes
```

## Usage

```rust
use paint_instructions::{PaintScene, PaintInstruction, PaintRect};
use paint_vm_gdi::render;

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

Windows only. On macOS use `paint-metal`, on Linux use `paint-vm-cairo` (future).
