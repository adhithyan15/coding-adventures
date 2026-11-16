# Changelog

## 0.1.1 - 2026-11-16

- Widen the `trig` dependency bound to `>=0.1 && <0.3` so the package builds against
  `trig` 0.2.0, which converges on the TRIG-REF-1 algorithm of PHY00a (TC-12). Results that
  pass through `sin`, `cos`, `tan`, `radians` or `degrees` can differ in the last bits.

## 0.1.0 - 2026-07-19

- Add SVG endpoint and center representations for elliptical arcs.
- Add W3C endpoint-to-center conversion with radius correction and flag logic.
- Add evaluation, tangents, exact rotated-ellipse arc bounds, and cubic Bezier
  approximation through the pure Haskell geometry dependency chain.
- Add package-native tests for degenerate cases, sweep choices, radius scaling,
  rotation, tight bounds, cubic segmentation, continuity, and control points.
