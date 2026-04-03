# arc2d

Elliptical arcs with endpoint↔center form conversion and cubic Bezier approximation.

## Overview

- **`CenterArc`**: center, rx, ry, start_angle, sweep_angle, x_rotation. Methods: evaluate, tangent, bounding_box, to_cubic_beziers.
- **`SvgArc`**: SVG `A` command parameters. Methods: to_center_arc, evaluate, bounding_box, to_cubic_beziers.

## Layer

G2D03 — depends on `point2d` (G2D00), `bezier2d` (G2D02), and `trig` (PHY00).
