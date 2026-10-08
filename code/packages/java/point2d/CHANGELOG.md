# Changelog

## [Unreleased]

### Added

- First Java G2D00 Point/Rect implementation with immutable value records,
  vector arithmetic, normalization at the exact `1e-12` boundary, PHY00
  `Trig.atan2` angle routing, and half-open rectangle operations.
- Native JUnit tests for the required G2D00 behaviors and all four checked
  `geometry2d-v1` point-normalization fixtures, including exact origins.
- Local trig composite-build dependency, capability declaration, and a 95%
  line-coverage gate for the package's cross-platform BUILD fronts.
