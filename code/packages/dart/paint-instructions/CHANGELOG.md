# Changelog

All notable changes to `coding_adventures_paint_instructions` will be documented
in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

- `PaintLine` — a straight stroked line segment (x1, y1, x2, y2, stroke,
  strokeWidth)
- `PaintGlyphPlacement` — a single glyph (`glyphId`, `x`, `y`); for the
  ASCII backend, `glyphId` is a literal Unicode code point rather than a
  font-internal glyph index (see `P2D02-paint-vm-ascii.md`)
- `PaintGlyphRun` — a run of `PaintGlyphPlacement`s sharing a font
  reference, size, and fill
- `Transform2D` — a 2D affine transform (`a, b, c, d, e, f`) with a static
  `identity` constant and an `isIdentity` getter
- `PaintGroup` — a list of child instructions sharing an optional
  transform and opacity
- `PaintClip` — a list of child instructions clipped to a rectangle
- `PaintLayer` — a list of child instructions sharing optional filters,
  blend mode, opacity, and transform
- `stroke` and `strokeWidth` fields on `PaintRect`, defaulting to `''` and
  `0.0` (fully backward compatible — existing `PaintRect(...)` call sites
  and the `paintRect()` helper are unaffected)
- `paintLine()`, `paintGlyphRun()`, `paintGroup()`, `paintClip()`, and
  `paintLayer()` builder helpers, matching the existing `paintRect()`/
  `paintPath()` helper conventions
- These additions bring `PaintInstruction` to the full `P2D02-paint-vm-ascii.md`
  contract (rect/line/glyph_run/group/clip/layer), consumed by the new
  `coding_adventures_paint_vm_ascii` package and the Dart `cowsay` port
  (see `code/specs/cowsay-paintvm-pipeline.md`)

## [0.1.0] — 2026-04-24

### Added

- `PathCommand` sealed class with three concrete subtypes:
  - `MoveTo` — lift the pen and move to (x, y)
  - `LineTo` — draw a straight line to (x, y)
  - `Close` — close the current sub-path back to the most recent move_to
- `PaintInstruction` sealed base class for all renderable shapes
- `PaintRect` — axis-aligned filled rectangle (x, y, width, height, fill)
- `PaintPath` — filled polygon described by a list of `PathCommand`s
- `PaintScene` — complete render frame (width, height, background,
  instructions, metadata)
- `PaintColorRGBA8` — parsed RGBA color with one byte per channel
- `paintRect()` helper with sane defaults (fill="#000000", metadata={})
- `paintPath()` helper with sane defaults
- `createScene()` helper with sane defaults (background="#ffffff", metadata={})
- `parseColorRGBA8()` — parse CSS hex strings (#rgb, #rgba, #rrggbb, #rrggbbaa)
- `version` constant: `"0.1.0"`
- 30+ unit tests covering all public API surface, including error paths
