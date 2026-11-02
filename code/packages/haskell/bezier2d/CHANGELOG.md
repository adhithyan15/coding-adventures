# Changelog

## Unreleased

- Replace midpoint-only recursive flattening with bounded, iterative G2D02
  subdivision using finite-segment control-point error.
- Return deterministic `FlatteningError` values for invalid tolerance,
  non-finite controls or derived geometry, and exhausted depth or split budgets.
- Consume all nine neutral flattening cases dynamically and retain native
  Bezier2D plus downstream Arc2D regression coverage.

## 0.1.0 - 2026-07-19

- Add immutable G2D02 quadratic and cubic Bezier values.
- Add de Casteljau evaluation and exact subdivision.
- Add unnormalized derivatives, adaptive polyline flattening, and tight
  derivative-root bounding boxes.
- Add exact quadratic-to-cubic degree elevation.
- Add package-native tests covering the complete required G2D02 behavior and
  degenerate bounding-box cases.
