# Changelog

## [0.3.0] - 2026-04-03

### Added

- `Sqrt(x float64)` — square root via Newton's method; panics for negative inputs.
- `Tan(x float64)` — tangent as Sin/Cos ratio with pole guard.
- `Atan(x float64)` — arctangent via Taylor series with outer and half-angle range reduction.
- `Atan2(y, x float64)` — four-quadrant arctangent.
- `halfPI` private constant.
- `atanCore(x float64)` private helper for inner atan computation.
- Tests for all new functions in `trig_test.go`.

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All four public functions (`Sin`, `Cos`,
  `Radians`, `Degrees`) are now wrapped with `StartNew[float64]` from the
  package's Operations infrastructure. Each call gains automatic timing,
  structured logging, and panic recovery.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `Sin(x)` via Maclaurin series with range reduction
- `Cos(x)` via Maclaurin series with range reduction
- `Radians(deg)` degree-to-radian conversion
- `Degrees(rad)` radian-to-degree conversion
