# Changelog

## Unreleased

### Fixed
- SVG endpoint arcs with near-zero radii or endpoints now evaluate along the endpoint line and return ordered endpoint bounds, including reversed and coincident endpoints. Center conversion remains optional and degenerate cubic output remains empty.
- Align degenerate guards with G2D03: absolute radius `<1e-10` and endpoint distance squared `<1e-20`, with native execution of all four shared geometry fixtures.

## [0.1.0] - 2026-04-02

### Added
- `CenterArc` struct: new, evaluate, tangent, bounding_box, to_cubic_beziers
- `SvgArc` struct: new, to_center_arc (W3C algorithm), evaluate, bounding_box, to_cubic_beziers
- Degenerate arc detection (same endpoints, zero radius)
- 100-point sampling for bounding box approximation
- Standard cubic Bezier approximation for ≤90° arc segments
