# CodingAdventures.Arc2D.CSharp

Elliptical arcs with SVG endpoint conversion, center-form evaluation, bounds, tangents, and cubic Bezier approximation.

Degenerate `SvgArc` values retain a nullable `ToCenterArc()` and empty cubic
output, while `Evaluate(t)` yields endpoint-line interpolation and
`BoundingBox()` yields an ordered endpoint rectangle. Coincident endpoints
produce a point rectangle; negative radii use their SVG absolute values.
