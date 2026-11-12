# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

## 0.1.0 - 2026-07-19

- Add immutable G2D00 `Point` and `Rect` values.
- Add vector arithmetic, products, norms, interpolation, perpendiculars, and
  direction angles through the existing pure Haskell `trig` package.
- Add half-open containment, union, strict positive-area intersection, and
  symmetric rectangle expansion.
- Add package-native coverage for construction, vector geometry, empty and
  negative rectangles, boundary behavior, and set operations.
