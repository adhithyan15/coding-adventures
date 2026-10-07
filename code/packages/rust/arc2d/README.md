# arc2d

Elliptical arcs with endpoint↔center form conversion and cubic Bezier approximation.

## Overview

- **`CenterArc`**: center, rx, ry, start_angle, sweep_angle, x_rotation. Methods: evaluate, tangent, bounding_box, to_cubic_beziers.
- **`SvgArc`**: SVG `A` command parameters. Methods: to_center_arc, evaluate, bounding_box, to_cubic_beziers.

For a degenerate endpoint arc (coincident endpoints or a radius with magnitude
below `1e-10`), `to_center_arc()` is `None` and cubics are empty, but
`evaluate(t)` returns the line interpolation and `bounding_box()` returns an
ordered endpoint rect. The public optional return types are retained.

## Layer

G2D03 — depends on `point2d` (G2D00), `bezier2d` (G2D02), and `trig` (PHY00).
