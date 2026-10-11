# Changelog

## Unreleased

### Fixed
- Normalize vectors with magnitude below `1e-12` to a fresh origin, including nonzero near-zero vectors; exactly `1e-12` remains normalizable (G2D00).

## [0.1.0] - 2026-04-02

### Added
- add, subtract, scale, negate, dot, cross, magnitude, magnitude_squared, normalize, distance, lerp, perpendicular, angle
- new_rect, contains_point, rect_union, rect_intersection, rect_expand
