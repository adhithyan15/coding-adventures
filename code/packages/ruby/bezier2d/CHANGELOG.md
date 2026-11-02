# Changelog

## Unreleased

- Adopt the nine-case G2D02 safe-flattening corpus with finite-segment bounds,
  iterative depth/work budgets, and fail-closed numeric validation.
- Run every native Bezier2D test file on Unix and Windows build fronts.

## [0.1.0] - 2026-04-02

### Added
- QuadraticBezier: eval, deriv, split, polyline, bbox, elevate
- CubicBezier: eval, deriv, split, polyline, bbox
- De Casteljau algorithm for stable evaluation
- Tight bounding box via derivative root finding
