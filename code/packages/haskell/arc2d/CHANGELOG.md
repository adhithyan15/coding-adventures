# Changelog

## 0.1.0 - 2026-07-19

- Add SVG endpoint and center representations for elliptical arcs.
- Add W3C endpoint-to-center conversion with radius correction and flag logic.
- Add evaluation, tangents, exact rotated-ellipse arc bounds, and cubic Bezier
  approximation through the pure Haskell geometry dependency chain.
- Add package-native tests for degenerate cases, sweep choices, radius scaling,
  rotation, tight bounds, cubic segmentation, continuity, and control points.
