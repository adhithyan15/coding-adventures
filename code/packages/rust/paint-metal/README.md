# paint-metal

Metal GPU renderer for the paint-instructions scene model — spec P2D01.

Takes a `PaintScene`, lowers it through the shared `paint-vm-gpu-core` command
plan, and renders it to a `PixelContainer` using Apple's Metal GPU API. Shaped
CoreText glyph runs become transient textures, so vectors, images, gradients,
clips, and glyphs retain painter order.

## Requirements

- macOS, Apple Silicon (arm64)
- Metal-capable GPU (all Apple Silicon Macs)

## Usage

```rust
use paint_metal;
use paint_instructions::{PaintScene, PaintInstruction, PaintRect, PaintEllipse, PaintBase};
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
// Render the ordered GPU command stream
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
| `PaintGroup`      | Shared-plan transforms and inherited opacity                   |
| `PaintClip`       | Nested rectangular Metal scissor clips                         |
| `PaintEllipse`    | Shared indexed GPU mesh                                        |
| `PaintPath`       | Shared flattened fill/stroke GPU meshes                        |
| `PaintText`       | Requires producer shaping; layout scenes emit glyph runs       |
| `PaintGlyphRun`   | Ordered CoreText raster texture for `coretext:` fonts          |
| `PaintLayer`      | Isolated offscreen surfaces, ordered filters, post-filter opacity, and all shared blend modes |
| `PaintGradient`   | Shared linear/radial texture ramps                             |
| `PaintImage`      | Ordered textures with affine transform, clip, opacity, scaling |

All 2D barcode formats (QR Code, Data Matrix, Aztec, PDF417) produce only
`PaintRect` instructions — the current implementation is complete for that use case.

## Architecture

```text
PaintScene
  │
  ├─ paint-vm-gpu-core
  │    ordered meshes + image/gradient uploads + clip commands
  │
  ├─ optional host GpuImageResolver
  │    URI → decoded PixelContainer (host owns policy and caching)
  │
  ├─ CoreText glyph rasterization
  │    one transparent texture per ordered shaped run
  │
  └─ Metal render and compute passes
       ordered meshes + nested scissors + ping-pong isolated layers
       + filter/composite kernels → RGBA8 readback
```

`render_with_image_resolver` and
`render_to_metal_layer_with_image_resolver` accept caller-supplied URI policy.
The default `render` path remains deterministic and expects decoded
`PaintImage::Pixels`, as Venture's browser resource pipeline already provides.
Layer filters execute in declaration order on transparent offscreen textures.
Composition uses separate source, backdrop, and destination textures so nested
layers never depend on in-place texture read/write behavior. Metal shader and
command-buffer failures include the native compiler/runtime diagnostic.

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
