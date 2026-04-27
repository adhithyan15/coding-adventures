# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-24

### Added

- Initial Lua port of the shared 2D barcode abstraction layer
- `make_module_grid(rows, cols [, module_shape])` -- creates an all-light
  ModuleGrid table (1-indexed, immutable-by-convention)
- `set_module(grid, row, col, dark)` -- pure functional single-module update;
  only the affected row is re-allocated
- `layout(grid [, config])` -- converts a ModuleGrid to a PaintScene;
  dispatches to square or hex rendering based on module_shape
- Square rendering: one paint_rect per dark module, plus one background rect
- Hex rendering: one paint_path (7-command flat-top hexagon) per dark module,
  with odd-row offset tiling for MaxiCode compatibility
- DEFAULT_CONFIG table with documented defaults
- Validation errors (InvalidBarcode2DConfigError) for invalid config values
  and mismatched module shape
- paint_path support added to coding-adventures-paint-instructions
- Full busted test suite with 50 test cases covering all public functions and
  internal helpers
