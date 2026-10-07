# arc2d (Python)

Elliptical arcs. G2D03 — depends on `point2d`, `bezier2d`, `trig`.

For a degenerate `SvgArc`, `to_center_arc()` is `None` and cubic output is
empty. `evaluate(t)` nevertheless follows the endpoint line, and
`bounding_box()` returns an ordered endpoint rectangle (a point rectangle
when endpoints coincide). Radii use SVG absolute-value semantics before the
strict `<1e-10` guard.
