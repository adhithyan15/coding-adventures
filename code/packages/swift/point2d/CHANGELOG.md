# Changelog

## Unreleased

### Fixed
- Normalize vectors with magnitude below `1e-12` to the origin, including nonzero near-zero vectors; exactly `1e-12` remains normalizable (G2D00).

## [0.1.0] - 2026-04-02

### Added
- Point: add, subtract, scale, negate, dot, cross, magnitudeSquared, magnitude, normalize, distanceSquared, distance, lerp, perpendicular, angle
- Rect: containsPoint, union, intersection, expandedBy
