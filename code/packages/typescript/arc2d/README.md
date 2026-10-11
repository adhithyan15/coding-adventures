# @coding-adventures/arc2d

Elliptical arcs with endpoint↔center form conversion.

`SvgArc.toCenterArc()` returns `null` for coincident endpoints or a radius
whose magnitude is below `1e-10`. `evaluate(t)` still returns the endpoint
line interpolation, and `boundingBox()` returns an ordered endpoint rectangle;
`toCubicBeziers()` returns an empty list. The nullable API shapes remain for
existing callers.

## Layer

G2D03 — depends on `@coding-adventures/point2d`, `@coding-adventures/bezier2d`, and `trig`.
