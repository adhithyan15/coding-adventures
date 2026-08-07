# Changelog

## [Unreleased]

### Changed

- Return `Atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

### Fixed

- Preserve negative zero and both signs of the minimum subnormal in `Atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-15

### Added

- First-principles `Trig` implementation covering `sin`, `cos`, `tan`,
  `sqrt`, `atan`, `atan2`, `Radians`, and `Degrees`
- Literate inline documentation explaining Maclaurin series, range reduction,
  and Newton iteration
- Unit tests covering canonical identities, quadrant handling, and conversion
  round-trips
