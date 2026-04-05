# paint-metal

Metal GPU renderer for the paint-instructions scene model — spec P2D01.

Takes a `PaintScene` and renders it to a `PixelContainer` using Apple's Metal GPU API.
This is the GPU renderer in the `paint-*` stack, replacing the older `draw-instructions-metal`
crate. The key difference is that it operates on `PaintScene` (f64 pixel coords) and
returns `PixelContainer` instead of `PixelBuffer`.

## Requirements

- macOS, Apple Silicon (arm64)
- Metal-capable GPU (all Apple Silicon Macs)

## Usage

```rust
use paint_metal;
use paint_instructions::{PaintScene, PaintInstruction, PaintRect};
use paint_codec_png::encode_png;

// Build a scene (or get one from barcode_2d::layout)
let mut scene = PaintScene::new(400.0, 400.0);
scene.instructions.push(PaintInstruction::Rect(
    PaintRect::filled(10.0, 10.0, 380.0, 380.0, "#000000"),
));

// Render on GPU
let pixels = paint_metal::render(&scene);   // → PixelContainer

// Encode to PNG
let png_bytes = encode_png(&pixels);
std::fs::write("output.png", &png_bytes).unwrap();
```

## Barcode pipeline

```rust
let grid = qr_code::encode("https://example.com", EccLevel::M);
let scene = barcode_2d::layout(&grid, &Barcode2DLayoutConfig::default());
let pixels = paint_metal::render(&scene);
let png = paint_codec_png::encode_png(&pixels);
std::fs::write("qr.png", png).unwrap();
```

## Instruction support

| Instruction       | Status                                        |
|-------------------|-----------------------------------------------|
| `PaintRect`       | Fully implemented — solid-colour rects        |
| `PaintLine`       | Fully implemented — rendered as thin rects    |
| `PaintGroup`      | Fully implemented — recurses into children    |
| `PaintClip`       | Partial — children rendered, no stencil yet   |
| `PaintGlyphRun`   | Planned (CoreText glyph rasterisation)        |
| `PaintEllipse`    | Planned (CPU tessellation → triangles)        |
| `PaintPath`       | Planned (CPU tessellation → triangles)        |
| `PaintLayer`      | Planned (offscreen texture + composite)       |
| `PaintGradient`   | Planned (MSL gradient shader)                 |
| `PaintImage`      | Planned (texture from PixelContainer or URI)  |

All 2D barcode formats (QR Code, Data Matrix, Aztec, PDF417) produce only
`PaintRect` instructions — the current implementation is complete for that use case.

## Architecture

```text
PaintScene
  → paint_metal::render()               (this crate)
  → PixelContainer                      (paint-instructions)

PixelContainer
  → paint_codec_png::encode_png()       (paint-codec-png)
  → Vec<u8> (PNG file bytes)
```

## Spec

P2D01 — `code/specs/P2D01-paint-vm.md` (dispatch-table VM spec)
