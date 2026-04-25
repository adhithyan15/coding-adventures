# Changelog — com.codingadventures:paint-instructions

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
