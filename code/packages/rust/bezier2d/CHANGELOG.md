# Changelog

## Unreleased

### Changed
- Make quadratic and cubic `to_polyline` return
  `Result<Vec<Point>, FlatteningError>`.
- Replace midpoint-only recursion with bounded iterative subdivision using the
  finite-segment control-point error bound.
- Reject invalid tolerance, non-finite input or derived geometry, and exhausted
  depth or split budgets atomically while consuming all nine G2D02 cases.

## [0.1.0] - 2026-04-02

### Added
- `QuadraticBezier` struct: new, evaluate, derivative, split, to_polyline, bounding_box, elevate
- `CubicBezier` struct: new, evaluate, derivative, split, to_polyline, bounding_box
- Adaptive subdivision for polyline approximation
- Tight bounding box via derivative root finding
- Comprehensive test suite including edge cases (straight lines, symmetric curves)
