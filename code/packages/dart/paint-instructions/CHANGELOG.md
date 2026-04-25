# Changelog

All notable changes to `coding_adventures_paint_instructions` will be documented
in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
