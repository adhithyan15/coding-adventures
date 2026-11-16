# Changelog

## [Unreleased]

### Changed

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

### Fixed

- Preserve negative zero and both signs of the minimum subnormal in `atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.

### Build

- `BUILD` and `BUILD_windows` now run the TRIG-REF-1 fixture generator's
  `--check` (byte-for-byte regeneration plus the sensitivity gates) and its validator
  tests (PHY00a 6.4 step 5). The package's own behaviour is unchanged: it does not
  read the fixture until slice TC-2 converges it. It is the package the build tool
  selects when the fixture, its generator, oracle or validator changes.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

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
