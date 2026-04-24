# Changelog — paint-instructions (Kotlin)

All notable changes to this package are documented here.
Follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
