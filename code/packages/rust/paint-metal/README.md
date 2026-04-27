# paint-metal

Metal GPU renderer for the paint-instructions scene model — spec P2D01.

Takes a `PaintScene` and renders it to a `PixelContainer` using Apple's Metal GPU API
plus a CoreText overlay for text instructions. This is the GPU renderer in the `paint-*`
stack, replacing the older `draw-instructions-metal` crate.

## Requirements

- macOS, Apple Silicon (arm64)
- Metal-capable GPU (all Apple Silicon Macs)

## Usage

```rust
use paint_metal;
use paint_instructions::{PaintScene, PaintInstruction, PaintRect, PaintEllipse, PaintText, PaintBase, TextAlign};
use paint_codec_png::encode_png;

// Build a diagram scene
let mut scene = PaintScene::new(400.0, 300.0);
scene.instructions.push(PaintInstruction::Rect(
    PaintRect::filled(10.0, 10.0, 380.0, 280.0, "#e8f4ff"),
));
scene.instructions.push(PaintInstruction::Ellipse(PaintEllipse {
    base: PaintBase::default(),
    cx: 200.0, cy: 150.0, rx: 80.0, ry: 50.0,
    fill: Some("#4a90d9".to_string()),
    stroke: Some("#2c5f8a".to_string()),
    stroke_width: Some(2.0),
}));
scene.instructions.push(PaintInstruction::Text(PaintText {
    base: PaintBase::default(),
    x: 200.0, y: 155.0,
    text: "My Node".to_string(),
    font_ref: Some("canvas:system-ui@14:400".to_string()),
    font_size: 14.0,
    fill: Some("#ffffff".to_string()),
    text_align: Some(TextAlign::Center),
}));

// Render on GPU (+ CoreText text overlay)
let pixels = paint_metal::render(&scene);   // → PixelContainer

// Encode to PNG
let png_bytes = encode_png(&pixels);
std::fs::write("diagram.png", &png_bytes).unwrap();
```

## Diagram pipeline (end-to-end)

```text
DOT source text
  → dot-parser                   (Rust, DG01)
  → GraphDiagram                 (diagram-ir, DG00)
  → diagram-layout-graph         (Rust, DG02)
  → LayoutedGraphDiagram         (diagram-ir, DG00)
  → diagram-to-paint             (Rust, DG03)
  → PaintScene                   (paint-instructions, P2D00)
  → paint_metal::render()        (this crate, P2D01)
  → PixelContainer
  → paint_codec_png::encode_png()
  → diagram.png
```

## Instruction support

| Instruction       | Status                                                        |
|-------------------|---------------------------------------------------------------|
| `PaintRect`       | Fully implemented — fill + 4-edge stroke quads                |
| `PaintLine`       | Fully implemented — rendered as thin perpendicular rectangle  |
| `PaintGroup`      | Fully implemented — recurses into children                    |
| `PaintClip`       | Partial — children rendered, no stencil buffer yet            |
| `PaintEllipse`    | Implemented — 64-triangle fan fill + 64-quad stroke ring      |
| `PaintPath`       | Implemented — fan fill (convex) + segment stroke + Bézier approx |
| `PaintText`       | Implemented — CoreText CTLine overlay into CGBitmapContext     |
| `PaintGlyphRun`   | Implemented — CoreText CTFontDrawGlyphs overlay               |
| `PaintLayer`      | Planned (offscreen texture + composite pass)                  |
| `PaintGradient`   | Planned (MSL gradient shader)                                 |
| `PaintImage`      | Planned (texture from PixelContainer or URI)                  |

All 2D barcode formats (QR Code, Data Matrix, Aztec, PDF417) produce only
`PaintRect` instructions — the current implementation is complete for that use case.

## Architecture

```text
PaintScene
  │
  ├─ GPU pass (Metal)
  │    collect_geometry() → triangle vertex buffers
  │    rect, line, ellipse (fan), path (fan + stroke segs)
  │    → PixelContainer (RGBA8)
  │
  ├─ CoreText overlay (PaintText)
  │    CTLineCreateWithAttributedString + CTLineDraw
  │    → drawn directly into CGBitmapContext wrapping pixel buffer
  │
  └─ CoreText overlay (PaintGlyphRun)
       CTFontDrawGlyphs (pre-positioned glyph IDs)
       → drawn directly into CGBitmapContext wrapping pixel buffer
```

## font_ref format (PaintText)

`PaintText.font_ref` uses the `"canvas:<family>@<size>:<weight>"` format (DG03 spec):

```
"canvas:system-ui@14:400"     →  Helvetica, 14pt
"canvas:monospace@12:700"     →  Courier Bold, 12pt
"canvas:Helvetica@18:400"     →  Helvetica, 18pt
```

Logical CSS family names are mapped to PostScript names:
- `system-ui`, `sans-serif`, `-apple-system` → `Helvetica`
- `monospace` → `Courier`
- `serif` → `Times-Roman`
- Any other name is passed through as-is (PostScript name)

## Spec

- P2D00 — `code/specs/P2D00-paint-instructions.md` (paint IR)
- P2D01 — `code/specs/P2D01-paint-vm.md` (dispatch-table VM spec)
- DG03 — `code/specs/DG03-diagram-to-paint.md` (diagram → PaintScene)
