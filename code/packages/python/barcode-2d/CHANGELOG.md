# Changelog

All notable changes to `coding-adventures-barcode-2d` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] - 2026-04-24

### Added

- `ModuleGrid` frozen dataclass — the universal intermediate representation
  for 2D barcode encoders.  Holds `rows`, `cols`, `modules` (tuple of tuples
  of booleans), and `module_shape` ("square" or "hex").

- `ModuleRole` — Literal type enumerating the eight structural roles a module
  can have: finder, separator, timing, alignment, format, data, ecc, padding.

- `ModuleAnnotation` frozen dataclass — optional per-module role metadata for
  visualizers.  Stores role, dark/light state, codeword_index, bit_index, and
  an arbitrary `metadata` dict for format-specific annotations.

- `AnnotatedModuleGrid` frozen dataclass — a `ModuleGrid` with a parallel
  `annotations` tuple for visualizer support.  Not required for rendering.

- `Barcode2DLayoutConfig` frozen dataclass — pixel-level rendering options
  with sensible defaults (10 px modules, 4-module quiet zone, black on white).

- `DEFAULT_BARCODE_2D_LAYOUT_CONFIG` — module-level constant with QR Code ISO
  18004 compliant defaults.

- `Barcode2DError` — base exception class for all barcode-2d errors.

- `InvalidBarcode2DConfigError` — raised by `layout()` for invalid config.

- `make_module_grid(rows, cols, module_shape)` — creates an all-light
  `ModuleGrid` of the given dimensions.  Starting point for every 2D barcode
  encoder.

- `set_module(grid, row, col, dark)` — pure immutable update; returns a new
  `ModuleGrid` with one module changed.  Only the affected row is reallocated;
  all other rows are shared with the original grid.  Raises `IndexError` for
  out-of-bounds coordinates.

- `layout(grid, config)` — converts a `ModuleGrid` to a `PaintScene`.
  Dispatches to `_layout_square` or `_layout_hex` based on `module_shape`.
  Validates config before dispatch.

- `_layout_square` — internal function for square-module grids.  Emits one
  background `PaintRectInstruction` then one `PaintRectInstruction` per dark
  module.

- `_layout_hex` — internal function for hex-module grids (MaxiCode).  Emits
  one background `PaintRectInstruction` then one `PaintPathInstruction` per
  dark module.  Uses flat-top hexagon geometry with circumradius
  `module_size_px / √3`.

- `_build_flat_top_hex_path` — internal geometry helper that produces the
  seven `PathCommand` objects (move_to, 5×line_to, close) for a flat-top
  regular hexagon at a given centre and circumradius.

### Changed

- `coding-adventures-paint-instructions`: added `PaintPathInstruction`,
  `PathCommand`, `paint_path()` as required by the hex-layout path.  The
  existing `PaintRectInstruction` and `paint_rect()` API is unchanged.
  All existing downstream packages continue to work without modification.

### Notes

- The spec (`barcode-2d.md`) describes only square-module layout using
  `PaintRect`.  The hex layout is an extension added by the TypeScript
  reference implementation and ported here.  This divergence is recorded
  in the TypeScript and Rust packages as well.
