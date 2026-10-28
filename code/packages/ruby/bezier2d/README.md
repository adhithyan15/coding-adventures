# bezier2d (Ruby)

Quadratic and cubic Bezier curves with evaluation, splitting, polyline, and bounding box. G2D02.

Polyline flattening validates finite inputs and a strictly positive tolerance,
uses a conservative finite-segment control-point bound, and performs bounded
left-to-right iterative subdivision. A depth, work, or numeric failure raises
without returning a partial polyline.
