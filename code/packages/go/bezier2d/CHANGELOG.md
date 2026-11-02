# Changelog

## Unreleased

### Changed
- Make `PolylineQuad` and `PolylineCubic` return an explicit error with their
  polyline result.
- Replace midpoint-only recursion with bounded iterative subdivision using the
  finite-segment control-point error bound.
- Reject invalid tolerance, non-finite input or derived geometry, and exhausted
  depth or split budgets atomically while consuming all nine G2D02 cases.

## [0.1.0] - 2026-04-02

### Added
- QuadraticBezier and CubicBezier types
- EvalQuad, DerivQuad, SplitQuad, PolylineQuad, BboxQuad, ElevateQuad
- EvalCubic, DerivCubic, SplitCubic, PolylineCubic, BboxCubic
- De Casteljau evaluation for numerical stability
- Adaptive polyline subdivision
- Tight bounding box via derivative root finding
- Degree elevation quadratic → cubic
