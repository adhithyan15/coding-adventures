# Changelog

## [Unreleased]

## [0.3.0] - 2026-11-16

### Changed

- **Numeric change (TRIG-CONV, PHY00a):** `sin`, `cos`, `radians` and `degrees` follow
  TRIG-REF-1 and match the shared vectors in `fixtures/trig-reference-v1` bit for bit:
  nearest-integer Cody-Waite reduction, ten-term Horner kernels on stored coefficients,
  constants from bit patterns, no division. The last bits of every result change and large
  arguments change materially; minor version bump past the existing 0.2.0 (owner decision OQ-12).
- **`sin`, `cos` and `tan` return NaN for `|x| > 2**29`, NaN and the infinities.**
- `sin(-0)` is now `-0`; `radians`/`degrees` are one multiply by a pre-divided constant;
  `atan2` returns NaN when either argument is NaN; `tan` keeps its `+-1e308` pole sentinel.

### Also released in this version (previously Unreleased)

#### Changed

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

#### Fixed

- Preserve negative zero and both signs of the minimum subnormal in `atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

#### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

### Tests

- `conformance` tests decode every row of `trig-reference-v1` (61,854 vectors, Part II
  included) by a bit cast and require identical result bits; constants' bits are asserted.

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
