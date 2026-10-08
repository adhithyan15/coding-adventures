# Changelog

## [Unreleased]

### Fixed
- Compute center-form arc bounds from directed, rotation-aware extrema instead of 100 samples.
- Emit exactly four cubic segments for a full turn and one degenerate segment for zero sweep.
- Explicitly reject invalid/non-finite center-form inputs and derived output, and sweeps beyond one turn, without changing public function signatures.

### Tests
- Consume all seven shared center-form bounds/cubic cases and native non-finite fail-stop cases; package statement coverage exceeds 95%.

## [0.1.0] - 2026-04-02

### Added
- CenterArc type: center, radii, start/sweep angle, x-rotation
- SvgArc type: SVG endpoint arc (A command) representation
- EvalArc: parametric evaluation at t ∈ [0,1]
- TangentArc: derivative / tangent vector
- BboxArc: sampling-based bounding box
- ToCubicBeziers: cubic Bezier approximation via k=(4/3)tan(s/4) formula
- ToCenterArc: W3C SVG §B.2.4 endpoint-to-center conversion
