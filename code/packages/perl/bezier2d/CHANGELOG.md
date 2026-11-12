# Changelog

## Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

- Adopt the nine-case G2D02 safe-flattening corpus with finite-segment bounds,
  iterative depth/work budgets, and fail-closed numeric validation.

## [0.1.0] - 2026-04-02

### Added
- QuadraticBezier: eval, deriv, split, polyline, bbox, elevate
- CubicBezier: eval, deriv, split, polyline, bbox
