# CodingAdventures.Arc2D.FSharp

Elliptical arcs with SVG endpoint conversion, center-form evaluation, bounds, tangents, and cubic Bezier approximation.

For degenerate `SvgArc` inputs, `ToCenterArc()` remains `None` and cubics are
empty, but `Evaluate(t)` returns `Some` endpoint-line interpolation and
`BoundingBox()` returns `Some` ordered endpoint rectangle. A coincident arc
has point bounds; SVG negative radii are corrected to absolute values.
