# Changelog — barcode-2d (Rust)

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `ModuleShape` enum — `Square` (QR Code, Data Matrix, Aztec, PDF417) or `Hex`
  (MaxiCode flat-top hexagonal grid). Implements `Clone`, `Debug`, `PartialEq`,
  `Eq`, and `Default` (defaults to `Square`).

- `ModuleGrid` struct — universal 2D boolean grid:
  `modules[row][col]` is `true` for a dark module, `false` for light.
  Fields: `cols: u32`, `rows: u32`, `modules: Vec<Vec<bool>>`,
  `module_shape: ModuleShape`.

- `ModuleRole` enum — generic module roles for all formats:
  `Finder`, `Separator`, `Timing`, `Alignment`, `Format`, `Data`, `Ecc`,
  `Padding`.

- `ModuleAnnotation` struct — per-module role annotation for visualizers.
  Includes `role`, `dark`, `codeword_index`, `bit_index`, and `metadata`
  (HashMap for format-specific role strings like `"qr:dark-module"`).

- `AnnotatedModuleGrid` struct — `ModuleGrid` plus a 2D `annotations` array
  (`Vec<Vec<Option<ModuleAnnotation>>>`).

- `Barcode2DLayoutConfig` struct — pixel-level rendering configuration:
  `module_size_px`, `quiet_zone_modules`, `foreground`, `background`,
  `show_annotations`, `module_shape`. Implements `Default` with QR-safe values.

- `make_module_grid(rows, cols, module_shape) -> ModuleGrid` — create an
  all-light grid.

- `set_module(grid, row, col, dark) -> ModuleGrid` — pure single-module update;
  panics on out-of-bounds (programming error guard).

- `layout(grid, config) -> Result<PaintScene, Barcode2DError>` — converts
  `ModuleGrid` → `PaintScene`:
  - Square modules: one `PaintRect` per dark module.
  - Hex modules: one `PaintPath` (flat-top hexagon) per dark module.
  - Validates config and returns `Err(InvalidConfig(…))` on bad input.

- `Barcode2DError` enum — `InvalidConfig(String)` and `DimensionMismatch(String)`.
  Implements `Display`, `Error`.

- `VERSION` constant — `"0.1.0"`.

- 41 unit tests covering all public functions and error paths.
