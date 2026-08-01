# Changelog — trig (Java)

## [Unreleased]

### Changed

- Return `Trig.atan(x)` unchanged for `|x| <= 2^-27` before half-angle
  reduction, preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

### Fixed

- Preserve negative zero and both signs of the minimum subnormal in `atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

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
