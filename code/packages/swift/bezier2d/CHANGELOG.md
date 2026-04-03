# Changelog — bezier2d (Swift)

## [0.1.0] - 2026-04-02

### Added
- `QuadraticBezier` struct with `eval`, `derivative`, `split`, `polyline`, `boundingBox`, `elevate`
- `CubicBezier` struct with `eval`, `derivative`, `split`, `polyline`, `boundingBox`
- De Casteljau algorithm for numerically stable evaluation and splitting
- Tight bounding box via derivative-zero root finding (quadratic formula for cubic)
- Adaptive polyline via chord-midpoint distance comparison
- 16 unit tests covering all operations
