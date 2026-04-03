# Changelog

## [0.2.0] - 2026-04-03

### Added

- `Trig.sqrt/1` — square root via Newton's method; raises `ArithmeticError` for negative inputs.
- `Trig.tan/1` — tangent as sin/cos ratio with pole guard.
- `Trig.atan/1` — arctangent via Taylor series with outer and half-angle range reduction.
- `Trig.atan2/2` — four-quadrant arctangent.
- `@half_pi` private module attribute.
- `atan_core/1` private helper using `Enum.reduce_while` for early termination.
- `sqrt_iterate/3` private tail-recursive helper for Newton iterations.
- Tests for all new functions in `test/trig_test.exs`.

## [0.1.0] - 2026-03-22

### Added
- `pi()` constant to double-precision accuracy
- `sin(x)` via Maclaurin series with range reduction
- `cos(x)` via Maclaurin series with range reduction
- `radians(deg)` degree-to-radian conversion
- `degrees(rad)` radian-to-degree conversion
