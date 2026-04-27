# Changelog — paint-instructions (Haskell)

All notable changes to this package are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
