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

## [0.2.0] - 2026-04-03

### Added

- `sqrt(x)` — square root via Newton's (Babylonian) iterative method; throws for negative inputs.
- `tan(x)` — tangent as sin/cos ratio with pole guard.
- `atan(x)` — arctangent via Taylor series with outer and half-angle range reduction.
- `atan2(y, x)` — four-quadrant arctangent.
- Tests for all new functions covering landmark values, roundtrips, and edge cases.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `sin(x)` via Maclaurin series with range reduction
- `cos(x)` via Maclaurin series with range reduction
- `radians(deg)` degree-to-radian conversion
- `degrees(rad)` radian-to-degree conversion
