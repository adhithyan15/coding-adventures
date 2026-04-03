# Changelog

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
