# Changelog

## [0.2.0] - 2026-04-03

### Added

- `sqrt(x)` — square root via Newton's (Babylonian) iterative method; raises `ValueError` for negative inputs.
- `tan(x)` — tangent as sin/cos ratio with pole guard (returns ±1e308 near singularities).
- `atan(x)` — arctangent via Taylor series with outer range reduction and half-angle reduction.
- `atan2(y, x)` — four-quadrant arctangent.
- `HALF_PI` module constant (π/2).
- `_atan_core(x)` private helper for the inner atan computation.
- Tests for all new functions.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `sin(x)` via Maclaurin series with range reduction
- `cos(x)` via Maclaurin series with range reduction
- `radians(deg)` degree-to-radian conversion
- `degrees(rad)` radian-to-degree conversion
