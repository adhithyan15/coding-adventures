# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries, with `paint-instructions` as a dependency) instead of skipping.

## [0.1.0] - 2026-04-24

### Added

- Initial Perl port of the shared 2D barcode abstraction layer
- `make_module_grid($rows, $cols, $module_shape)` — create an all-light ModuleGrid
- `set_module($grid, $row, $col, $dark)` — pure immutable single-module update
- `layout($grid, $config)` — convert ModuleGrid to PaintScene
- Square module rendering: each dark module becomes a `PaintRect`
- Hex module rendering (MaxiCode): each dark module becomes a flat-top `PaintPath`
- Full validation with descriptive croak messages for invalid config
- Added `paint_path` to `CodingAdventures::PaintInstructions` (required by hex rendering)
