# Changelog

## [0.2.0] - 2026-04-03

### Added

- `Trig.sqrt(x)` — square root via Newton's method; raises `ArgumentError` for negative inputs.
- `Trig.tan(x)` — tangent as sin/cos ratio with pole guard.
- `Trig.atan(x)` — arctangent via Taylor series with outer and half-angle range reduction.
- `Trig.atan2(y, x)` — four-quadrant arctangent.
- `HALF_PI` public constant.
- `atan_core(x)` private class method helper.
- Tests for all new functions in `test/test_trig.rb`.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `Trig.sin(x)` via Maclaurin series with range reduction
- `Trig.cos(x)` via Maclaurin series with range reduction
- `Trig.radians(deg)` degree-to-radian conversion
- `Trig.degrees(rad)` radian-to-degree conversion
