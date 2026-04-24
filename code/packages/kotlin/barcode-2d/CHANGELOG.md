# Changelog — barcode-2d (Kotlin)

All notable changes to this package are documented here.
Follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` enum — `SQUARE` (QR, Data Matrix, Aztec, PDF417) and `HEX`
  (MaxiCode flat-top hexagons).
- `ModuleGrid` data class — immutable 2D boolean grid representing the universal
  intermediate representation produced by every 2D barcode encoder.
- `ModuleRole` enum — `FINDER`, `SEPARATOR`, `TIMING`, `ALIGNMENT`, `FORMAT`,
  `DATA`, `ECC`, `PADDING` for annotated grids.
- `ModuleAnnotation` data class — per-module role metadata used by visualizers
  to colour-code barcode symbols.
- `AnnotatedModuleGrid` data class — `ModuleGrid` extended with a parallel 2D
  grid of nullable `ModuleAnnotation` values.
- `Barcode2DLayoutConfig` data class — pixel-level rendering options with
  sensible defaults: `moduleSizePx=10`, `quietZoneModules=4`,
  `foreground="#000000"`, `background="#ffffff"`.
- `Barcode2DException` open class — base for all barcode-2d errors.
- `InvalidBarcode2DConfigException` class — thrown when layout config is invalid.
- `makeModuleGrid()` — creates an all-light grid of given dimensions.
- `setModule()` — returns a new `ModuleGrid` with one module changed
  (copy-on-write; original is never modified).
- `layout()` — converts a `ModuleGrid` into a `PaintScene`, dispatching to
  `layoutSquare()` for square modules and `layoutHex()` for hex modules.
- `layoutSquare()` (internal) — renders square-module grids as `PaintRect`
  instructions.
- `layoutHex()` (internal) — renders MaxiCode hex grids as `PaintPath`
  instructions using flat-top hexagonal geometry.
- `buildFlatTopHexPath()` (internal) — builds the 7 `PathCommand`s for one
  flat-top regular hexagon.
- Full KDoc on every public type, function, and parameter.
- 60+ JUnit Jupiter unit tests covering all public API surface, defaults,
  error paths, square and hex layout geometry.
