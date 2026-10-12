# @coding-adventures/arc2d

Elliptical arcs with endpoint↔center form conversion.

`CenterArc.boundingBox()` uses analytic rotated extrema and includes both
endpoints, with signed modular membership for arcs crossing a full-turn seam.
Direct center forms require finite coordinates/angles and positive radii.
`evaluate`, `tangent`, `boundingBox`, and `toCubicBeziers` throw
`RangeError("arc2d: invalid center arc")` for invalid or non-finite inputs
and derived output; finite sweeps beyond one turn throw
`RangeError("arc2d: sweep exceeds one turn")`. Evaluation and tangent also
require finite `t`. Zero sweeps produce one degenerate cubic; a full turn
produces exactly four. The native suite consumes all seven neutral center
records in `geometry2d-v1` at runtime.

`SvgArc.toCenterArc()` returns `null` for coincident endpoints or a radius
whose magnitude is below `1e-10`. `evaluate(t)` still returns the endpoint
line interpolation, and `boundingBox()` returns an ordered endpoint rectangle;
`toCubicBeziers()` returns an empty list. The nullable API shapes remain for
existing callers.

## Layer

G2D03 — depends on `@coding-adventures/point2d`, `@coding-adventures/bezier2d`, and `trig`.
