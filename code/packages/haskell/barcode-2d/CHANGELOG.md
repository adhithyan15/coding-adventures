# Changelog — barcode-2d (Haskell)

All notable changes to this package are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` ADT: `Square` (QR, Data Matrix, Aztec, PDF417) and `Hex`
  (MaxiCode flat-top hexagons).

- `ModuleGrid` record backed by a flat `Data.Vector Bool` in row-major order,
  providing O(1) random access.  Fields: `mgRows`, `mgCols`, `mgModules`,
  `mgShape`.

- `emptyGrid rows cols shape` — create an all-False grid.

- `setModule grid row col dark` — copy-on-write single-module update using
  `V.//`.  Raises `error` on out-of-bounds access.

- `Barcode2DLayoutConfig` record with fields: `moduleSizePx`, `quietZoneModules`,
  `foreground`, `background`.

- `defaultConfig` — sensible defaults (10 px modules, 4-module quiet zone,
  black on white).

- `layout grid cfg` — public entry point.  Validates config fields (non-zero
  `moduleSizePx`, non-negative `quietZoneModules`) then dispatches to
  `layoutSquare` or `layoutHex` based on `mgShape`.

- `layoutSquare grid cfg` — renders square-module grids.  Produces one
  background `PaintRect` plus one `PaintRect` per dark module.

- `layoutHex grid cfg` — renders hex-module grids.  Produces one background
  `PaintRect` plus one `PaintPath` per dark module. Odd rows are offset right
  by `moduleSizePx / 2`.

- `buildHexPath cx cy circumR` — internal geometry helper building the seven
  `PathCommand`s (`MoveTo`, 5x `LineTo`, `ClosePath`) for a flat-top regular
  hexagon.

- Full Haddock documentation on all exported symbols with ASCII diagrams,
  geometry formulas, and worked examples.

- HSpec test suite covering:
  - `emptyGrid` construction and shape storage
  - `setModule` immutability, correct index update, out-of-bounds errors
  - `defaultConfig` field values
  - `layoutSquare` dimensions, instruction counts, pixel positioning, colors
  - `layoutHex` path count, path structure (7 commands, MoveTo/LineTo/ClosePath)
  - `layout` dispatch to square vs. hex
  - `layout` validation (rejects invalid `moduleSizePx` / `quietZoneModules`)
  - 21x21 QR-like grid sanity check
