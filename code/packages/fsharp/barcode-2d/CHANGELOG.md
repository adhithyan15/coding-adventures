# Changelog — CodingAdventures.Barcode2D (F#)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` discriminated union: `Square` | `Hex`
- `ModuleGrid` record: `Rows`, `Cols`, `Modules` (bool[][] immutable),
  `ModuleShape`
- `AnnotatedModuleGrid` record: wraps `ModuleGrid` with `Roles` (string option[]
  array) for per-module role annotations used by visualizers
- `Barcode2DLayoutConfig` record with fields `ModuleSizePx`, `QuietZoneModules`,
  `DarkColor`, `LightColor`
- `Barcode2D.defaultConfig` — sensible defaults (size=10, qz=4, black/white)
- `Barcode2D.makeModuleGrid` — create an all-light grid of given dimensions
- `Barcode2D.setModule` — pure immutable single-module update (structural
  sharing — only the affected row is re-allocated)
- `Barcode2D.layout` — converts `ModuleGrid` → `PaintScene`:
  - Square path: background rect + one `PaintRect` per dark module
  - Hex path: background rect + one flat-top hexagon `PaintPath` per dark
    module (offset-row tiling for MaxiCode)
- Validation in `layout`: raises `ArgumentException` for `ModuleSizePx <= 0`
  or `QuietZoneModules < 0`
- 57 xUnit tests covering all public API paths, edge cases, and geometry
- Line coverage target: 90%
