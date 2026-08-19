# Changelog — com.codingadventures:paint-instructions

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- `PaintInstruction` gains five new permitted subtypes, bringing the sealed
  hierarchy up to the set the new `paint-vm-ascii` backend needs to
  implement the full `P2D02-paint-vm-ascii.md` contract:
  - `PaintLine(x1, y1, x2, y2, stroke, strokeWidth, metadata)` — a stroked line segment.
  - `PaintGlyphRun(List<PaintGlyphPlacement> glyphs, fontRef, fontSize, fill, metadata)`
    — pre-positioned glyphs, plus the new `PaintGlyphPlacement` value type.
  - `PaintGroup(children, Optional<Transform2D>, Optional<Double> opacity, metadata)`.
  - `PaintClip(x, y, width, height, children, metadata)`.
  - `PaintLayer(children, hasFilters, Optional<String> blendMode, Optional<Double> opacity, Optional<Transform2D>, metadata)`.
- `PaintRect` gains `stroke`/`strokeWidth` fields via a new constructor
  overload; every existing constructor and call site is unaffected (the
  original 5-arg/6-arg constructors still default to no stroke).
- New `Transform2D` value type (six-value affine transform, Canvas/SVG
  convention) with an `IDENTITY` constant and `isIdentity()`.
- New builder helpers on `PaintInstructions`: `paintLine`, `paintGlyphRun`,
  `paintGroup`, `paintClip`, `paintLayer`, and a `paintRect` overload that
  accepts a stroke.
- This package is now consumed by the Java `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).

## [0.1.0] — 2026-04-24

### Added

- `PathCommand` — sealed abstract class with three permitted subtypes:
  - `MoveTo(double x, double y)` — lift pen and move without drawing
  - `LineTo(double x, double y)` — draw a straight line to `(x, y)`
  - `ClosePath` (singleton) — close the current sub-path back to the last `MoveTo`
- `PaintInstruction` — sealed abstract class with two permitted subtypes:
  - `PaintRect(int x, int y, int width, int height, String fill, Map<String,String> metadata)` — filled axis-aligned rectangle
  - `PaintPath(List<PathCommand> commands, String fill, Map<String,String> metadata)` — filled closed polygon
- `PaintScene` — immutable top-level container: `width`, `height`, `background`,
  `List<PaintInstruction> instructions`, `Map<String,String> metadata`
- `PaintInstructions` utility class with static builder helpers:
  - `paintRect(x, y, width, height, fill[, metadata])` — builds a `PaintRect`, defaulting fill to `#000000`
  - `paintPath(commands, fill[, metadata])` — builds a `PaintPath`, defaulting fill to `#000000`
  - `createScene(width, height, background, instructions[, metadata])` — builds a `PaintScene`, defaulting background to `#ffffff`
- Full JUnit Jupiter test suite covering construction, equality, immutability,
  sealed-class pattern matching, and round-trip builder usage
