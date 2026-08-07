# Changelog

## 0.1.0 - 2026-07-19

- Add the immutable G2D01 six-scalar `Affine` value.
- Add translation, rotation, centered rotation, scaling, and skew factories
  through the existing pure Haskell `point2d` and `trig` packages.
- Add ordered composition, point and vector application, determinant, checked
  inversion, tolerance predicates, and graphics-array conversion.
- Add package-native coverage for every required G2D01 behavior, including
  non-commutativity, singular matrices, centered rotation, and vector handling.
