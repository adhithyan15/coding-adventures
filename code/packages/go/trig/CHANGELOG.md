# Changelog

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
