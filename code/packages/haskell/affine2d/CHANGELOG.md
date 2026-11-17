# Changelog

## 0.1.1 - 2026-11-16

- Widen the `trig` dependency bound to `>=0.1 && <0.3` so the package builds against
  `trig` 0.2.0, which converges on the TRIG-REF-1 algorithm of PHY00a (TC-12). Results that
  pass through `sin`, `cos`, `tan`, `radians` or `degrees` can differ in the last bits.
- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

## 0.1.0 - 2026-07-19

- Add the immutable G2D01 six-scalar `Affine` value.
- Add translation, rotation, centered rotation, scaling, and skew factories
  through the existing pure Haskell `point2d` and `trig` packages.
- Add ordered composition, point and vector application, determinant, checked
  inversion, tolerance predicates, and graphics-array conversion.
- Add package-native coverage for every required G2D01 behavior, including
  non-commutativity, singular matrices, centered rotation, and vector handling.
