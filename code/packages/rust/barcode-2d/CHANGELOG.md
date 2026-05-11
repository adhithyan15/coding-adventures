# Changelog — barcode-2d (Rust)

## 0.2.0 — 2026-05-11

### Added

- **Native rendering pipeline** — `barcode-2d` is now the shared rendering hub
  for all 2D barcode formats. Any `PaintScene` produced by `encode_and_layout()`
  in `qr-code`, `pdf417`, `aztec-code`, `data-matrix`, or `micro-qr` can be
  passed directly to the render functions below.

- `current_backend() -> &'static str` — reports the platform-default backend
  name (`"direct2d"`, `"metal"`, `"cairo"`, or `"skia"`).

- `render_scene(scene: &PaintScene) -> Result<PixelContainer, String>` —
  renders using the platform-default backend. Priority order:
  Windows → Direct2D, macOS → Metal, Linux/BSD → Cairo, all others → Skia.

- `render_scene_png(scene: &PaintScene) -> Result<Vec<u8>, String>` —
  convenience wrapper: render → PNG bytes. The standard entry point for
  per-format wrappers in `qr-code`, `pdf417`, etc.

- `render_scene_with_backend(scene: &PaintScene, backend: &str) -> Result<PixelContainer, String>` —
  explicit backend selection. Accepts `"skia"`, `"cairo"`, `"metal"`,
  `"direct2d"`. Returns `Err` for unknown names or unavailable backends.

- `render_scene_png_with_backend(scene: &PaintScene, backend: &str) -> Result<Vec<u8>, String>` —
  explicit backend + PNG output. Useful for CI tests that pin a backend.

### Changed

- `VERSION` bumped to `"0.2.0"`.
- Description updated to reflect the crate's dual role as layout hub AND
  rendering hub.

### Dependencies added

- `paint-codec-png` — PNG encoding.
- `paint-vm-runtime` — `PaintRenderError` type.
- `paint-vm-skia` — Skia CPU raster backend (all platforms).
- `paint-vm-cairo` — Cairo backend (macOS, Linux, BSD; system `cairo` required).
- `paint-metal` — Metal GPU backend (macOS only).
- `paint-vm-direct2d` — Direct2D GPU backend (Windows only).

### Tests added (3 unconditional + 1 platform-gated)

- `current_backend_returns_known_name` — backend name is one of the 4 known strings.
- `skia_renders_tiny_square_to_png` — Skia output is a valid PNG on all platforms.
- `unknown_backend_returns_error` — descriptive error on bad backend name.
- `cairo_renders_tiny_square_to_png` — Cairo PNG is valid (macOS/Linux/BSD only).
- `platform_default_render_produces_png` — default dispatcher produces valid PNG.

**Total tests: 44 (43 unit + 1 doctest).**

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
