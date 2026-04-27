# Changelog

All notable changes to `coding_adventures_barcode_2d` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-24

### Added

- `ModuleGrid` struct: immutable 2D boolean grid (`cols`, `rows`, `modules`, `module_shape`).
- `ModuleRole` constants: `FINDER`, `SEPARATOR`, `TIMING`, `ALIGNMENT`, `FORMAT`, `DATA`, `ECC`, `PADDING`.
- `DEFAULT_BARCODE_2D_LAYOUT_CONFIG` frozen hash with sensible defaults (`module_size_px: 10`, `quiet_zone_modules: 4`, `foreground: "#000000"`, `background: "#ffffff"`, `module_shape: "square"`).
- `Barcode2D.make_module_grid(rows, cols, module_shape: "square")` — factory for all-light grids.
- `Barcode2D.set_module(grid, row, col, dark)` — pure immutable single-module update; raises `RangeError` for out-of-bounds.
- `Barcode2D.layout(grid, config = nil)` — converts a `ModuleGrid` into a `PaintScene`:
  - Square-module path: one `PaintRect` per dark module plus a background rect.
  - Hex-module path (MaxiCode): one `PaintPath` (flat-top hexagon, 7 commands) per dark module.
  - Raises `InvalidBarcode2DConfigError` for `module_size_px <= 0`, `quiet_zone_modules < 0`, or `config[:module_shape]` / `grid.module_shape` mismatch.
- `Barcode2DError` and `InvalidBarcode2DConfigError` error classes.
- 74 minitest unit tests covering all public methods, edge cases, and error paths.
- `standardrb` linting passes with no offences.
- `paint_path` builder added to `coding_adventures_paint_instructions` to support hex-module rendering.
