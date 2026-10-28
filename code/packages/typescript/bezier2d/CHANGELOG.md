# Changelog

## Unreleased

- Adopt the nine-case G2D02 safe-flattening corpus with finite-segment bounds,
  iterative depth/work budgets, and fail-closed numeric validation.
- Correct the Unix build front so dependency setup cannot change the package
  under test, and raise the production line-coverage floor to 95%.

## [0.1.0] - 2026-04-02

### Added
- `QuadraticBezier` and `CubicBezier` classes with full evaluate/split/toPolyline/boundingBox
