# Changelog

## [Unreleased]

### Fixed

- Added `BUILD_windows`. The Unix `BUILD` calls `.venv/bin/python`, which does not
  exist on Windows (the interpreter is `.venv\Scripts\python.exe`), so the package
  failed in the first Windows full build with `'.venv' is not recognized as an
  internal or external command`. The new file runs the same steps with Windows
  paths; no source change.

## 0.1.0 — 2026-04-27

### Added
- Initial implementation of Data Matrix ECC200 encoder (ISO/IEC 16022:2006).
- ASCII encoding with digit-pair compaction.
- Symbol selection across all 30 square and 6 rectangular Data Matrix sizes.
- Reed-Solomon error correction over GF(256)/0x12D with b=1 convention.
- Utah diagonal codeword placement with L-finder and timing borders.
- `encode(data, size, shape)` — encode string to `ModuleGrid`.
- `encode_at(data, rows, cols)` — encode to specific symbol size.
- `layout_grid(grid)` — convert `ModuleGrid` to `PaintScene`.
- `encode_and_layout(data)` — convenience wrapper.
- `grid_to_string(grid)` — debug/snapshot text rendering.
- `SymbolShape` constants (`Square`, `Rectangle`, `Any`).
- `DataMatrixError`, `InputTooLongError`, `InvalidSymbolError` exception hierarchy.
