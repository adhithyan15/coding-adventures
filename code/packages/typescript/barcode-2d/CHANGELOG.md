# Changelog — @coding-adventures/barcode-2d

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `ModuleGrid` interface — universal 2D boolean grid representation for all
  2D barcode formats (QR Code, Data Matrix, Aztec Code, PDF417, MaxiCode).
  `modules[row][col]` is `true` for a dark module, `false` for light.

- `ModuleShape` type — `"square"` (QR/Data Matrix/Aztec/PDF417) or `"hex"`
  (MaxiCode flat-top hexagonal grid).

- `ModuleRole` type — generic module roles applicable across all formats:
  `"finder"`, `"separator"`, `"timing"`, `"alignment"`, `"format"`, `"data"`,
  `"ecc"`, `"padding"`.

- `ModuleAnnotation` interface — per-module role annotation for visualizers,
  including optional `codewordIndex`, `bitIndex`, and `metadata` fields for
  format-specific role strings (e.g. `"qr:dark-module"`).

- `AnnotatedModuleGrid` interface — `ModuleGrid` extended with a 2D
  `annotations` array; used by drill-down visualizers, not required for
  rendering.

- `Barcode2DLayoutConfig` interface — pixel-level rendering configuration:
  `moduleSizePx`, `quietZoneModules`, `foreground`, `background`,
  `showAnnotations`, `moduleShape`.

- `DEFAULT_BARCODE_2D_LAYOUT_CONFIG` constant — sensible defaults matching QR
  Code's minimum quiet-zone requirement (4 modules, 10 px/module).

- `makeModuleGrid(rows, cols, moduleShape?)` — create an all-light grid.

- `setModule(grid, row, col, dark)` — immutable single-module update; throws
  `RangeError` for out-of-bounds coordinates.

- `layout(grid, config?)` — convert `ModuleGrid` → `PaintScene`:
  - Square modules: one `PaintRect` per dark module.
  - Hex modules: one `PaintPath` (flat-top hexagon) per dark module.
  - Validates config and throws `InvalidBarcode2DConfigError` on bad input.

- `Barcode2DError` base error class.
- `InvalidBarcode2DConfigError` for layout validation failures.

- Re-exports `PaintScene` from `@coding-adventures/paint-instructions` so
  callers can type `layout()`'s return value without importing paint-instructions
  directly.
