# paint-instructions

Universal 2D paint intermediate representation (IR) for Rust — spec P2D00.

This crate defines the complete type system shared between scene producers
(barcodes, charts, diagrams) and rendering backends (Metal GPU, SVG, terminal).
It is the Rust counterpart of the TypeScript `@coding-adventures/paint-instructions` package.

## What's in here

| Type | Role |
|---|---|
| `PixelContainer` | RGBA8 pixel buffer — output of renderers, input to image codecs |
| `ImageCodec` | Encode/decode trait implemented by `paint-codec-png`, `paint-codec-webp`, … |
| `PaintScene` | Top-level scene: viewport size, background, ordered instruction list |
| `PaintInstruction` | Enum of all instruction types |
| `PaintRect` | Filled/stroked rectangle |
| `PaintEllipse` | Filled/stroked ellipse or circle |
| `PaintPath` | Arbitrary vector path from `PathCommand`s |
| `PaintGlyphRun` | Pre-positioned glyphs from a font (post-shaping) |
| `PaintGroup` | Logical container for transform/opacity; renders to parent surface |
| `PaintLayer` | Offscreen compositing surface with filters and blend modes |
| `PaintLine` | Straight line segment |
| `PaintClip` | Rectangular clip mask |
| `PaintGradient` | Linear or radial colour gradient |
| `PaintImage` | Raster image from URI or `PixelContainer` |

## Pipeline

```rust
// Barcode → PaintScene → Metal GPU → PixelContainer → PNG bytes
let grid = qr_code::encode("https://example.com", EccLevel::M);
let scene = barcode_2d::layout(&grid, &Barcode2DLayoutConfig::default());
let pixels = paint_metal::render(&scene);          // PaintScene → PixelContainer
let png = paint_codec_png::PngCodec.encode(&pixels); // PixelContainer → Vec<u8>
std::fs::write("qr.png", png).unwrap();
```

## Relationship to `draw-instructions-pixels`

`PixelContainer` is the successor to `draw-instructions-pixels::PixelBuffer`.
Same RGBA8 row-major layout; the key difference is that it lives in the `paint-*`
stack (P2D00 onward) rather than the legacy `draw-instructions-*` stack.

## Architecture

```text
Producer (barcode, chart)
  → PaintScene / PaintInstruction    ← this crate (paint-instructions)
  → paint-metal / paint-vm-svg       (P2D01 backends)
  → PixelContainer / SVG string

PixelContainer
  → paint-codec-png::PngCodec.encode()    → PNG bytes
  → paint-codec-webp::WebpCodec.encode()  → WebP bytes
```

## Spec

P2D00 — `code/specs/P2D00-paint-instructions.md`
