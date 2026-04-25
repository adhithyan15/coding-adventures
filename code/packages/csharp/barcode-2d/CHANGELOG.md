# Changelog — CodingAdventures.Barcode2D

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` enum (`Square`, `Hex`) — identifies the geometric shape of
  each module in a grid.
- `ModuleGrid` class — immutable 2D boolean grid, the universal
  intermediate representation produced by every 2D barcode encoder.
  - `ModuleGrid.Create(rows, cols, moduleShape)` — factory method, all modules
    start light (`false`).
  - `ModuleGrid.SetModule(row, col, dark)` — pure immutable update; only the
    affected row is re-allocated (structural sharing).
  - `ArgumentOutOfRangeException` on out-of-bounds coordinates.
- `ModuleRole` enum — structural roles for visualizers (`Finder`, `Separator`,
  `Timing`, `Alignment`, `Format`, `Data`, `Ecc`, `Padding`).
- `ModuleAnnotation` record — per-module role annotation with optional
  `CodewordIndex`, `BitIndex`, and `Metadata` escape hatch.
- `AnnotatedModuleGrid` — extends `ModuleGrid` with a parallel annotation
  layer; optional and never inspected by the renderer.
- `Barcode2DLayoutConfig` record — pixel-level rendering options with
  sensible defaults via `Barcode2DLayoutConfig.Default`
  (`ModuleSizePx=10`, `QuietZoneModules=4`, `Foreground="#000000"`,
  `Background="#ffffff"`, `ModuleShape=Square`).
- `Barcode2DException` / `InvalidBarcode2DConfigException` — error hierarchy
  for invalid configuration.
- `Barcode2D` static class:
  - `Layout(grid, config?)` — validates config and dispatches to the correct
    renderer; the primary public API.
  - `LayoutSquare(grid, config?)` — square-module renderer (QR Code, Data
    Matrix, Aztec Code, PDF417). Emits one background `PaintRect` plus one
    `PaintRect` per dark module.
  - `LayoutHex(grid, config?)` — flat-top hex-module renderer (MaxiCode).
    Emits one background `PaintRect` plus one `PaintPath` (7 commands:
    MoveTo + 5×LineTo + ClosePath) per dark module. Odd rows are offset right
    by `hexWidth / 2` to produce the standard hexagonal tiling.
- 71 xUnit tests covering: `VERSION`, `ModuleGrid.Create`, immutability and
  structural sharing in `SetModule`, bounds checking, config defaults,
  annotation types, validation errors, square pixel geometry, hex geometry
  (dimensions, path structure, odd-row offset), `Layout` dispatch, and edge
  cases (1×1 grid, all-dark grid, QR v40 177×177, PDF417-style wide grid,
  MaxiCode 33×30).
