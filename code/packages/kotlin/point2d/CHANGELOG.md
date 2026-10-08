# Changelog

## [Unreleased]

### Added

- First Kotlin G2D00 Point/Rect implementation as immutable data values,
  including vector laws, the strict `<1e-12` normalization guard, local PHY00
  `Trig.atan2`, and half-open rectangle union/intersection.
- Native JUnit tests loading exactly the four checked Point2D cases from the
  mixed `geometry2d-v1` corpus, with exact origin comparisons.
- Composite trig dependency, capability declaration, and 95% JaCoCo line
  verification in both BUILD fronts.
