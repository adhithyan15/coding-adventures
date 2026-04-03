# Changelog

## [0.2.0] - 2026-04-03

### Added

- `pub fn sqrt(x: f64)` — square root via Newton's method; panics for negative inputs.
- `pub fn tan(x: f64)` — tangent as sin/cos ratio with pole guard.
- `pub fn atan(x: f64)` — arctangent via Taylor series with outer and half-angle range reduction.
- `pub fn atan2(y: f64, x: f64)` — four-quadrant arctangent.
- `HALF_PI` private constant.
- `fn atan_core(x: f64)` private helper for inner atan computation.
- Tests for all new functions in `tests/trig_tests.rs`.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `sin(x)` via Maclaurin series with range reduction
- `cos(x)` via Maclaurin series with range reduction
- `radians(deg)` degree-to-radian conversion
- `degrees(rad)` radian-to-degree conversion
