# Changelog — com.codingadventures:barcode-2d

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` enum with `SQUARE` and `HEX` values
- `ModuleGrid` — immutable 2D boolean grid:
  - Constructor defensively copies and wraps both outer and inner lists as unmodifiable
  - `equals()`, `hashCode()`, `toString()` following value-object convention
- `InvalidBarcode2DConfigException` — `RuntimeException` thrown by `layout()` on bad config
- `Barcode2DLayoutConfig` — immutable config with:
  - Defaults: `moduleSizePx=10`, `quietZoneModules=4`, `foreground="#000000"`, `background="#ffffff"`, `moduleShape=SQUARE`
  - Static `defaults()` factory
  - Fluent `Builder` inner class
  - `equals()`, `hashCode()`, `toString()`
- `Barcode2D` utility class:
  - `makeModuleGrid(rows, cols[, moduleShape])` — create all-light grid
  - `setModule(grid, row, col, dark)` — pure/immutable copy-on-write update
  - `layout(grid[, config])` — validates config and dispatches to square or hex renderer
  - `layoutSquare(grid, config)` — internal square-module renderer (package-private for testing)
  - `layoutHex(grid, config)` — internal hex-module renderer (package-private for testing)
  - `buildFlatTopHexPath(cx, cy, circumR)` — flat-top hexagon geometry helper
- Full JUnit Jupiter test suite covering all public and package-private methods,
  validation, geometry, and full-pipeline integration tests
- Composite build via `settings.gradle.kts` pulling in `paint-instructions` locally
