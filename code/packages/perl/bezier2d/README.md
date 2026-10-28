# bezier2d (Perl)

Quadratic and cubic Bezier curves. G2D02.

Polyline flattening validates finite inputs and a strictly positive tolerance,
uses a conservative finite-segment control-point bound, and performs bounded
left-to-right iterative subdivision. A depth, work, or numeric failure dies
without returning a partial polyline.
