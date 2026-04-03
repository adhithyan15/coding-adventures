# Changelog

## [0.1.0] - 2026-04-02

### Added
- `Point` struct with full 2D vector arithmetic: add, subtract, scale, negate, dot, cross, magnitude, magnitude_squared, normalize, distance, distance_squared, lerp, perpendicular, angle
- `Rect` struct with AABB operations: new, from_points, zero, min_point, max_point, center, is_empty, contains_point, union, intersection, expand_by
- Comprehensive test suite covering all edge cases (zero vector, unit vectors, boundary conditions)
