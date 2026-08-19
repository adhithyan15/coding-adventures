# Changelog — paint-instructions (Kotlin)

All notable changes to this package are documented here.
Follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `PaintInstruction` gains five new permitted subtypes, bringing the sealed
  hierarchy up to the set the new `paint-vm-ascii` backend needs to
  implement the full `P2D02-paint-vm-ascii.md` contract:
  - `PaintLine` — a stroked line segment.
  - `PaintGlyphRun` — pre-positioned glyphs (`PaintGlyphPlacement`), plus
    the new `PaintGlyphPlacement` data class itself.
  - `PaintGroup` — a child list with an optional `Transform2D` and opacity.
  - `PaintClip` — a rectangular clip region wrapping a child list.
  - `PaintLayer` — a child list with a filter flag, blend mode, opacity,
    and transform.
- `PaintRect` gains `stroke`/`strokeWidth` fields with default values
  (`""`/`0.0`), so every existing call site keeps working unchanged.
- New `Transform2D` data class (six-value affine transform, Canvas/SVG
  convention) with an `IDENTITY` constant and `isIdentity()`.
- New builder helpers: `paintLine`, `paintGlyphRun`, `paintGroup`,
  `paintClip`, `paintLayer`.
- This package is now consumed by the Kotlin `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).

## [0.1.0] — 2026-04-24

### Added

- `PaintColorRGBA8` data class — 32-bit RGBA colour with 8 bits per channel.
- `PathCommand` sealed class — `MoveTo`, `LineTo`, `ClosePath` drawing commands.
- `PaintInstruction` sealed class — `PaintRect` (filled axis-aligned rectangle)
  and `PaintPath` (filled closed polygon) instruction subtypes.
- `PaintScene` data class — canvas dimensions, background colour, and ordered
  instruction list.
- `paintRect()` helper — constructs a `PaintInstruction.PaintRect` with sensible
  defaults (fill defaults to `#000000`).
- `paintPath()` helper — constructs a `PaintInstruction.PaintPath` with sensible
  defaults (fill defaults to `#000000`).
- `createScene()` helper — constructs a `PaintScene` with sensible defaults
  (background defaults to `#ffffff`).
- `parseColorRGBA8()` — parses CSS hex colour strings (`#rgb`, `#rgba`,
  `#rrggbb`, `#rrggbbaa`) into `PaintColorRGBA8`.
- `Metadata` type alias — `Map<String, String>` for arbitrary annotations.
- Full KDoc on every public type, function, and parameter.
- 38 JUnit Jupiter unit tests covering all public API surface, defaults, error
  paths, and sealed class branches.
