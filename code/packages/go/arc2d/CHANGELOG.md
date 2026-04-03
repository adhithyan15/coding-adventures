# Changelog

## [0.1.0] - 2026-04-02

### Added
- CenterArc type: center, radii, start/sweep angle, x-rotation
- SvgArc type: SVG endpoint arc (A command) representation
- EvalArc: parametric evaluation at t ∈ [0,1]
- TangentArc: derivative / tangent vector
- BboxArc: sampling-based bounding box
- ToCubicBeziers: cubic Bezier approximation via k=(4/3)tan(s/4) formula
- ToCenterArc: W3C SVG §B.2.4 endpoint-to-center conversion
