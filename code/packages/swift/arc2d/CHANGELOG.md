# Changelog — arc2d (Swift)

## [0.1.0] - 2026-04-02

### Added
- `CenterArc` struct with `eval`, `boundingBox` (100-point sampling), `toCubicBeziers`
- `SvgArc` struct with `toCenterArc` (W3C SVG §B.2.4 algorithm)
- k = (4/3)·tan(s/4) formula for cubic Bézier arc approximation
- Automatic segmentation: arcs > 90° are split into multiple segments
- 13 unit tests covering eval, bbox, Bézier conversion, and SVG conversion
