# Changelog

## Unreleased

### Fixed
- Compute tight center-form bounds from analytic rotated extrema, including signed wraparound and zero sweeps.
- Reject invalid/non-finite center forms, derived output and sweeps beyond one turn before cubic allocation; preserve one zero-sweep cubic and four full-turn cubics.
- Consume all seven neutral center-form records and test native non-finite inputs, overflow, signed full turns and independent rotated containment.
- Endpoint-form degenerate arcs now yield line-evaluated points and ordered bounds while center conversion stays nullable and cubic output stays empty.
- Match G2D03's strict absolute-radius and squared-endpoint thresholds; run the four shared geometry fixtures in the native suite.

## [0.1.0] - 2026-04-02

### Added
- `CenterArc` and `SvgArc` classes with full conversion and approximation
