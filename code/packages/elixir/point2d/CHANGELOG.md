# Changelog

## Unreleased

### Fixed
- Normalize vectors with magnitude below `1e-12` to the origin, including nonzero near-zero vectors; exactly `1e-12` remains normalizable (G2D00).

## [0.1.0] - 2026-04-02

### Added
- Point functions: add, subtract, scale, negate, dot, cross, magnitude, magnitude_squared, normalize, distance, distance_squared, lerp, perpendicular, angle
- Rect functions: contains_point?, rect_union, rect_intersection, rect_expand
