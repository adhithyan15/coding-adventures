# Changelog — paint-instructions

## 0.1.1 — 2026-04-23

### Added

- `PaintText` struct — simple string-plus-position text instruction for backends with native text support (Metal, SVG, Canvas); counterpart to the existing `PaintGlyphRun` which requires pre-shaped glyph IDs
  - Fields: `x`, `y`, `text`, `font_ref`, `font_size`, `fill`, `text_align`
  - `font_ref` uses `"canvas:<family>@<size>:<weight>"` format per DG03 spec
- `TextAlign` enum — `Left` (default), `Center`, `Right`
- `PaintInstruction::Text(PaintText)` variant added between `Path` and `GlyphRun`
- Tests: `paint_text_fields_round_trip`, `text_align_default_is_left`, `paint_instruction_text_variant`

## 0.1.0 — 2026-04-05

Initial release.

### Added

- `PixelContainer` — RGBA8 pixel buffer; successor to `draw-instructions-pixels::PixelBuffer`
- `ImageCodec` trait — encode/decode interface for `paint-codec-png`, `paint-codec-webp`, etc.
- `PaintScene` — top-level scene container with viewport, background, and instruction list
- `PaintInstruction` enum — union of all 10 instruction types
- `PaintRect` — filled/stroked rectangle with optional `corner_radius`
- `PaintEllipse` — filled/stroked ellipse or circle
- `PaintPath` with `PathCommand` — arbitrary vector paths (MoveTo, LineTo, QuadTo, CubicTo, ArcTo, Close)
- `PaintGlyphRun` + `GlyphPosition` — pre-positioned font glyphs
- `PaintGroup` — logical grouping with optional transform and opacity
- `PaintLayer` — offscreen compositing surface with `FilterEffect` and `BlendMode`
- `PaintLine` — straight line segment
- `PaintClip` — rectangular clip mask
- `PaintGradient` with `GradientKind` (Linear / Radial) and `GradientStop`
- `PaintImage` with `ImageSrc` (URI or `PixelContainer`) — raster image instruction
- `Transform2D` type alias (`[f64; 6]`) + `IDENTITY_TRANSFORM` constant
- `BlendMode` enum (16 modes: Normal, Multiply, Screen, …)
- `FilterEffect` enum (Blur, DropShadow, ColorMatrix, Brightness, Contrast, Saturate, HueRotate, Invert, Opacity)
- `FillRule`, `StrokeCap`, `StrokeJoin` enums
- `PaintBase` — shared optional `id` and `metadata` fields
- `PaintRect::filled()` convenience constructor
- `PaintScene::new()` convenience constructor
- Full test suite covering `PixelContainer`, `PaintRect`, `PaintScene`, `PaintInstruction` variants
