# Changelog — trig (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of trigonometric functions from first principles.
- `Trig.sin(x)` — 20-term Maclaurin series with range reduction to [-π, π].
- `Trig.cos(x)` — 20-term Maclaurin series with range reduction to [-π, π].
- `Trig.tan(x)` — implemented as `sin(x)/cos(x)` with pole guard.
- `Trig.sqrt(x)` — Newton's (Babylonian) method; throws `ArithmeticException` for negative input.
- `Trig.atan(x)` — Taylor series with two-layer range reduction (|x|>1 and half-angle).
- `Trig.atan2(y, x)` — four-quadrant arctangent with correct quadrant handling.
- `Trig.radians(deg)` / `Trig.degrees(rad)` — angle unit conversion.
- `Trig.PI` — π constant to full double precision.
- 57 unit tests covering special values, symmetry identities, Pythagorean identity,
  large-input range reduction, roundtrip conversions, and all four atan2 quadrants.
