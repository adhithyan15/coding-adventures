# Changelog

## 0.1.1 - 2026-11-16

- Widen the `trig` dependency bound to `>=0.1 && <0.3` so the package builds against
  `trig` 0.2.0, which converges on the TRIG-REF-1 algorithm of PHY00a (TC-12). Results that
  pass through `sin`, `cos`, `tan`, `radians` or `degrees` can differ in the last bits.
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
