# bezier2d (Python)

Quadratic and cubic Bezier curves. G2D02 — depends on `point2d` (G2D00).

Polyline flattening validates finite inputs and a strictly positive tolerance,
uses a conservative finite-segment control-point bound, and performs bounded
left-to-right iterative subdivision. A depth, work, or numeric failure raises
without returning a partial polyline.
