# Changelog — bezier2d (Swift)

## Unreleased

### Changed
- Make quadratic and cubic `polyline(tolerance:)` throwing, iterative, and
  conservative over finite endpoint segments.
- Enforce finite positive tolerance, finite control and derived geometry,
  depth 32, and 65,535-subdivision budgets with typed `FlatteningError` values.
- Consume all nine neutral flattening cases dynamically and raise source line
  coverage to 98.21% while retaining downstream Arc2D tests.

## [0.1.0] - 2026-04-02

### Added
- `QuadraticBezier` struct with `eval`, `derivative`, `split`, `polyline`, `boundingBox`, `elevate`
- `CubicBezier` struct with `eval`, `derivative`, `split`, `polyline`, `boundingBox`
- De Casteljau algorithm for numerically stable evaluation and splitting
- Tight bounding box via derivative-zero root finding (quadratic formula for cubic)
- Adaptive polyline via chord-midpoint distance comparison
- 16 unit tests covering all operations
