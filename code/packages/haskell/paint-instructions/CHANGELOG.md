# Changelog — paint-instructions (Haskell)

All notable changes to this package are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `PaintInstruction` gains five new constructors, bringing the ADT up to the
  set the `paint-vm-ascii` backend needs to implement the full
  `P2D02-paint-vm-ascii.md` contract (this Haskell port previously only had
  `PaintRect`/`PaintPath`, unlike every other language's `paint-instructions`):
  - `PaintGlyphRun` — pre-positioned glyphs (`PaintGlyphPlacement`), plus the
    new `PaintGlyphPlacement` record itself.
  - `PaintLine` — a stroked line segment between two points.
  - `PaintGroup` — a child list with an optional `Transform2D` and opacity.
  - `PaintClip` — a rectangular clip region wrapping a child list.
  - `PaintLayer` — a child list with a filter flag, blend mode, opacity, and
    transform (simplified from the general filter-effect union: this repo's
    Haskell backend has no filter-rendering path, so only whether filters
    are present matters).
- `PaintRect` gains `prStroke` and `prStrokeWidth` fields, so rectangles can
  be stroked (box-drawing outline) as well as filled.
- New `Transform2D` record and `identityTransform` constant.
- New builder helpers: `makeGlyphRun`, `makeLine`, `makeGroup`, `makeClip`,
  `makeLayer` (mirroring the existing `makeRect`/`makePath` convention).
- This package is now consumed by the Haskell `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).

## [0.1.0] — 2026-04-24

### Added

- `PathCommand` ADT with `MoveTo`, `LineTo`, and `ClosePath` constructors.
  Covers all path shapes needed by 2D barcode renderers (squares, hexagons).

- `PaintInstruction` ADT:
  - `PaintRect` — filled rectangle with position, size, CSS fill color, and
    optional metadata.
  - `PaintPath` — arbitrary path built from `[PathCommand]`, CSS fill color,
    and optional metadata.

- `PaintScene` record:
  - `psWidth`, `psHeight` — canvas dimensions in user-space units.
  - `psBg` — CSS background color painted before all instructions.
  - `psInstructions` — ordered list of drawing commands (back-to-front).
  - `psMeta` — optional scene-level metadata forwarded unchanged.

- Builder helpers:
  - `emptyScene w h bg` — create a scene with no instructions.
  - `makeRect x y w h fill` — create a `PaintRect` with empty metadata.
  - `makePath cmds fill` — create a `PaintPath` with empty metadata.
  - `addInstruction scene instr` — pure append returning a new scene.

- Full Haddock documentation on every exported symbol, with ASCII diagrams
  and worked examples throughout (literate-programming style).

- HSpec test suite covering:
  - `PathCommand` construction and equality
  - `PaintRect` and `PaintPath` field correctness
  - `PaintScene` structure and defaults
  - All four builder helpers
  - Mixed instruction types in a single scene
  - Immutability of `addInstruction`
