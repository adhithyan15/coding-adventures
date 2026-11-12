# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

- Add strict v1 binary, width, layout, and scene facades while preserving the
  legacy calling conventions used by downstream symbologies.
- Dynamically execute all 56 language-neutral fixtures, including stable
  failures, checked limits, Unicode, metadata ownership, and text-first
  zero-authority evidence.
- Declare an empty production capability profile.

## [0.1.0] - 2026-04-12

### Added

- Initial 1D barcode layout primitives and PaintScene translation
