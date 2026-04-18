# Changelog — @coding-adventures/paint-instructions

## Unreleased

### Fixed

- import the `PixelContainer` type locally as well as re-exporting it so sibling TypeScript packages can type-check against `paintImage()` and `PaintImage.src`

## 0.1.0 — 2026-04-03

Initial release implementing the P2D00 PaintInstructions IR spec.

### Added

- `PaintInstruction` union type covering all 10 instruction kinds:
  `rect`, `ellipse`, `path`, `glyph_run`, `group`, `layer`, `line`, `clip`, `gradient`, `image`
- `PaintBase` interface with optional `id` and `metadata` fields on every instruction
- `PathCommand` union for tracing arbitrary vector paths (`move_to`, `line_to`, `quad_to`, `cubic_to`, `arc_to`, `close`)
- `Transform2D` — 6-element affine matrix matching `CanvasRenderingContext2D.transform()` argument order
- `FilterEffect` union — 9 filter types: `blur`, `drop_shadow`, `color_matrix`, `brightness`, `contrast`, `saturate`, `hue_rotate`, `invert`, `opacity`
- `BlendMode` — 16 compositing modes (separable and non-separable, matching CSS/SVG spec)
- `PixelContainer` — raw pixel buffer interface (output of `PaintVM.export()`, input to `ImageCodec.encode()`)
- `ImageCodec` — encode/decode interface for image format packages (`paint-codec-png`, etc.)
- `PaintScene` — top-level container with viewport, background, and ordered instruction list
- Builder helper functions: `paintScene`, `paintRect`, `paintEllipse`, `paintPath`, `paintLine`, `paintGroup`, `paintLayer`, `paintClip`, `paintGradient`, `paintImage`
