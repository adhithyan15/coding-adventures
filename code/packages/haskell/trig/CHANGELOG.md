# Changelog

## [Unreleased]

### Changed

- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

### Fixed

- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

### Tests

- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

## 0.1.0 - 2026-07-18

- Add first-principles sine and cosine using range-reduced Maclaurin series.
- Add degree/radian conversion and Newton-method square root.
- Add tangent with pole guards plus arctangent and four-quadrant arctangent.
