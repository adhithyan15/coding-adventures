# arc2d (Go)

Elliptical arc in center form and SVG endpoint form, with W3C conversion. G2D03.

Center-form bounds use analytic rotated-ellipse extrema and consume the shared
`geometry2d-v1` bounds and cubic-count fixtures. A direct `CenterArc` requires
finite center coordinates, angles, and positive finite radii, with at most one
turn of sweep. `EvalArc`, `TangentArc`, `BboxArc`, and `ToCubicBeziers` preserve
their value-returning signatures: invalid or non-finite input/output panics
with `ErrInvalidCenterArc`, while an over-turn sweep panics with
`ErrInvalidSweep`. A caller can recover and compare the panic value with
`errors.Is`; SVG endpoint conversion remains a separate API.
