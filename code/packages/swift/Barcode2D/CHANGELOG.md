# Changelog — Barcode2D (Swift)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` enum with `.square` and `.hex` cases.
- `ModuleRole` enum with eight structural roles: `.finder`, `.separator`,
  `.timing`, `.alignment`, `.format`, `.data`, `.ecc`, `.padding`.
- `ModuleGrid` struct — the universal 2D barcode intermediate representation.
  Stores a `rows × cols` boolean grid of dark/light modules, plus module shape.
- `ModuleAnnotation` struct — per-module role metadata with optional
  `codewordIndex`, `bitIndex`, and format-specific `metadata` dictionary.
- `AnnotatedModuleGrid` struct — a `ModuleGrid` paired with a parallel
  `[[ModuleAnnotation?]]` grid for use by visualizers.
- `Barcode2DLayoutConfig` struct — rendering configuration with defaults:
  `moduleSizePx=10.0`, `quietZoneModules=4`, `foreground="#000000"`,
  `background="#ffffff"`, `showAnnotations=false`, `moduleShape=.square`.
- `Barcode2DError` enum with `.invalidConfig(String)` case.
- `makeModuleGrid(rows:cols:moduleShape:)` — creates an all-light grid.
- `setModule(grid:row:col:dark:)` — immutable (pure) module update; returns a
  new `ModuleGrid` with one module changed. Throws `.invalidConfig` on
  out-of-bounds access.
- `layout(grid:config:)` — converts a `ModuleGrid` to a `PaintScene`:
  - **Square path**: one `PaintRect` per dark module, preceded by a background
    rect covering the full symbol plus quiet zone.
  - **Hex path**: same rect-based approach but with hexagonal tiling geometry
    (odd rows offset by `hexWidth/2`), matching the P2D00 rect-only spec.
  - Validates `moduleSizePx > 0`, `quietZoneModules >= 0`, and
    `config.moduleShape == grid.moduleShape`.
- Package.swift, BUILD, BUILD_windows, README.md, CHANGELOG.md.
- Full XCTest suite with 40+ test cases covering all public APIs, edge cases,
  validation errors, and integration scenarios.

### Implementation notes

- The Swift `PaintInstructions` layer (P2D00) supports only `PaintRect`. The
  hex module path uses `PaintRect` approximations at hexagonal tiling positions,
  matching the spec requirement that the layout step use only P2D00 primitives.
  This diverges from the TypeScript implementation which uses `PaintPath` for
  hex — the Swift port aligns with the spec rather than the TS implementation.
- All pixel coordinates are computed as `Double` arithmetic and rounded to the
  nearest integer before being passed to `PaintRect`, which takes `Int` coords.
