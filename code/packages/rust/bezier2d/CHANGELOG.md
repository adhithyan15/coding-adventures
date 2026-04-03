# Changelog

## [0.1.0] - 2026-04-02

### Added
- `QuadraticBezier` struct: new, evaluate, derivative, split, to_polyline, bounding_box, elevate
- `CubicBezier` struct: new, evaluate, derivative, split, to_polyline, bounding_box
- Adaptive subdivision for polyline approximation
- Tight bounding box via derivative root finding
- Comprehensive test suite including edge cases (straight lines, symmetric curves)
