# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.1.0] - 2026-04-02

### Added
- CenterArc: eval_at, bbox, to_cubic_beziers
- SvgArc: to_center_arc (W3C SVG §B.2.4 algorithm)
