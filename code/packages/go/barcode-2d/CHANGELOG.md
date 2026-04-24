# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-24

### Added

- `ModuleGrid` struct: the universal intermediate representation for 2D barcode
  encoders. A rows × cols boolean grid where `true` = dark module and `false` =
  light module.
- `ModuleShape` enum: `ModuleShapeSquare` (QR, Data Matrix, Aztec, PDF417) and
  `ModuleShapeHex` (MaxiCode flat-top hexagons).
- `ModuleRole` enum: eight structural roles (`Finder`, `Separator`, `Timing`,
  `Alignment`, `Format`, `Data`, `Ecc`, `Padding`) common to all 2D barcode
  formats.
- `ModuleAnnotation` struct: per-module role metadata including `CodewordIndex`
  and `BitIndex` for visualizers that highlight individual codewords.
- `AnnotatedModuleGrid` struct: a `ModuleGrid` extended with a parallel
  `[][]*ModuleAnnotation` slice for teaching visualizers.
- `Barcode2DLayoutConfig` struct and `DefaultBarcode2DLayoutConfig` variable
  with sensible defaults (10 px modules, 4-module quiet zone, black on white).
- `MakeModuleGrid(rows, cols uint32, shape ModuleShape) ModuleGrid`: creates an
  all-light grid.
- `SetModule(grid ModuleGrid, row, col uint32, dark bool) (ModuleGrid, error)`:
  immutable single-module update using structural sharing (only the changed row
  is re-allocated).
- `Layout(grid ModuleGrid, config *Barcode2DLayoutConfig) (PaintScene, error)`:
  converts a `ModuleGrid` into pixel-level `PaintScene` instructions.
  - Square mode: one `PaintRect` per dark module.
  - Hex mode: one `PaintPath` (flat-top hexagon, 7 commands) per dark module.
  - Validates `ModuleSizePx > 0` and `config.ModuleShape == grid.ModuleShape`.
- `Barcode2DError` and `InvalidBarcode2DConfigError` error types with
  `IsInvalidBarcode2DConfigError(err)` helper.
- `Version` constant set to `"0.1.0"`.
- 42 unit tests with 100% statement coverage.

### Notes

- Depends on `github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions`.
- The `paint-instructions` package was extended in this PR to add
  `PathCommand`, `PaintPathInstruction`, and `PaintPath()` — required for
  hex-module rendering.
