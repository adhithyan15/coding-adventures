# Changelog — barcode-layout-1d

## Unreleased

- Added strict portable-v1 expansion, layout, and rectangle-scene entry points
  backed by all 56 language-neutral conformance vectors.
- Both portable-v1 text request forms now fail closed before backend dispatch,
  while the established legacy entry point retains native text rendering.
- Added checked limits, stable errors, canonical metadata, and deep-ownership
  coverage for the portable adapter.

## 0.1.0

- Shared `Barcode1DRun` model for Rust barcode symbologies
- Quiet-zone aware `compute_barcode_1d_layout()`
- `runs_from_binary_pattern()` and `runs_from_width_pattern()` helpers
- `layout_barcode_1d()` translation from barcode runs to `PaintScene`
