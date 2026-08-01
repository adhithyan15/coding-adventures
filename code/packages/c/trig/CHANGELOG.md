# Changelog

## [Unreleased]

### Changed

- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

### Fixed

- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

### Tests

- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

All notable changes to the C `trig` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `trig` crate — trigonometric functions
  computed from first principles with no `<math.h>` / libm.
- `trig_sin` / `trig_cos` (20-term Maclaurin series with range reduction into
  `[-pi, pi]`), `trig_tan` (sin/cos with pole saturation), `trig_atan` /
  `trig_atan2` (Taylor series with two-layer range reduction), `trig_radians` /
  `trig_degrees`, and `TRIG_PI`.
- `trig_sqrt` via Newton's method, using the `TrigStatus` out-parameter API
  (`TRIG_OK` / `TRIG_ERR_DOMAIN`) in place of the Rust panic on negative input.
- libm-free helpers for absolute value, truncation, and floating-point
  remainder (range reduction) so the package has zero external dependencies.
- 50 checks over sin/cos/tan/sqrt/atan/atan2/conversions (known values, parity,
  periodicity, the Pythagorean identity, and non-finite/NaN inputs), run under
  every available C compiler via the shared `iso-harness`.
- Range-reduction truncation guards NaN out of the `double`->`long long` cast
  (converting NaN to an integer is undefined behavior).
