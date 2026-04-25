# Changelog

All notable changes to `coding_adventures_barcode_2d` will be documented
in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-24

### Added

- `ModuleShape` enum with two variants:
  - `square` — axis-aligned rectangular modules (QR Code, Data Matrix, Aztec, PDF417)
  - `hex` — flat-top hexagonal modules (MaxiCode / ISO/IEC 16023)
- `ModuleRole` enum with eight structural roles for visualizer colour-coding:
  `finder`, `separator`, `timing`, `alignment`, `format`, `data`, `ecc`, `padding`
- `ModuleAnnotation` — per-module role annotation (role, dark, codewordIndex,
  bitIndex, metadata) for educational visualizers; never read by `layout()`
- `ModuleGrid` — immutable 2D boolean grid, the universal intermediate
  representation produced by every 2D barcode encoder
- `AnnotatedModuleGrid` — subclass of `ModuleGrid` extended with per-module
  `ModuleAnnotation?` list; `layout()` accepts it transparently
- `Barcode2DLayoutConfig` — value class for layout parameters:
  `moduleSizePx`, `quietZoneModules`, `foreground`, `background`, `moduleShape`
- `defaultBarcode2DLayoutConfig` — pre-built defaults
  (`moduleSizePx=10`, `quietZoneModules=4`, `foreground="#000000"`, `background="#ffffff"`,
  `moduleShape=square`)
- `Barcode2DLayoutConfig.copyWith()` — non-destructive field override helper
- `makeModuleGrid()` — construct an all-light `ModuleGrid` of given dimensions
- `setModule()` — pure immutable single-module update with structural sharing
- `layout()` — the only pixel-aware function in the barcode stack; converts a
  `ModuleGrid` into a `PaintScene` using the supplied `Barcode2DLayoutConfig`
  - Square path: emits one background `PaintRect` + one `PaintRect` per dark module
  - Hex path: emits one background `PaintRect` + one `PaintPath` (7 commands)
    per dark module using flat-top hexagon geometry with odd-row offset tiling
- `Barcode2DError` — base exception class for all errors from this library
- `InvalidBarcode2DConfigError` — thrown by `layout()` for invalid config or
  grid/config shape mismatch
- Re-export of `PaintScene` from `coding_adventures_paint_instructions` so callers
  can type the `layout()` return value without a second import
- `version` constant: `"0.1.0"`
- 60+ unit tests covering all public API surface, immutability guarantees,
  pixel geometry, error paths, and edge cases
