# Changelog

## Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Fixed
- Normalize vectors with magnitude below `1e-12` to a fresh origin, including nonzero near-zero vectors; exactly `1e-12` remains normalizable (G2D00).

## [0.1.0] - 2026-04-02

### Added
- Point: add, subtract, scale, negate, dot, cross, magnitude, magnitude_squared, normalize, distance, distance_squared, lerp, perpendicular, angle
- Rect: contains_point, union, intersection, expand_by
