# Changelog

## [0.1.0] - 2026-04-02

### Added
- `CenterArc` struct: new, evaluate, tangent, bounding_box, to_cubic_beziers
- `SvgArc` struct: new, to_center_arc (W3C algorithm), evaluate, bounding_box, to_cubic_beziers
- Degenerate arc detection (same endpoints, zero radius)
- 100-point sampling for bounding box approximation
- Standard cubic Bezier approximation for ≤90° arc segments
