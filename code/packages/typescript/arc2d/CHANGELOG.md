# Changelog

## Unreleased

### Fixed
- Endpoint-form degenerate arcs now yield line-evaluated points and ordered bounds while center conversion stays nullable and cubic output stays empty.
- Match G2D03's strict absolute-radius and squared-endpoint thresholds; run the four shared geometry fixtures in the native suite.

## [0.1.0] - 2026-04-02

### Added
- `CenterArc` and `SvgArc` classes with full conversion and approximation
