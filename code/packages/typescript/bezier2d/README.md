# @coding-adventures/bezier2d

Quadratic and cubic Bezier curves.

## Layer

G2D02 — depends on `@coding-adventures/point2d` (G2D00).

Polyline flattening validates finite inputs and a strictly positive tolerance,
uses a conservative finite-segment control-point bound, and performs bounded
left-to-right iterative subdivision. A depth, work, or numeric failure throws
without returning a partial polyline.
