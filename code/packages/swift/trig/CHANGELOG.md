# Changelog — trig (Swift)

## [Unreleased]

## [0.2.0] — 2026-11-16

### Changed

- **Numeric change (TRIG-CONV, PHY00a; TC-11):** `Trig.sin`, `Trig.cos`,
  `Trig.radians` and `Trig.degrees` follow TRIG-REF-1 and match the shared vectors in
  `fixtures/trig-reference-v1` bit for bit: nearest-integer Cody-Waite reduction
  (replaces `truncatingRemainder` plus clamping), ten-term Horner kernels on stored
  coefficients (replaces the 21-term recurrence with a division per term), constants from
  bit patterns. The last bits of every result change and large arguments change
  materially. Minor version bump from 0.1.0 (the Swift package has no manifest version;
  this is the CHANGELOG version).
- **`sin`, `cos` and `tan` return NaN for `|x| > 2^29`**, NaN and the infinities (they
  returned finite, wrong values for large arguments).
- `sin(-0)` is now `-0` (it was `+0`).
- `radians`/`degrees` multiply by the bit-pinned constants `RAD_PER_DEG`/`DEG_PER_RAD`
  (same bits as before for the previous `deg * (PI / 180)` form; now pinned).
- `Trig.atan2` returns NaN when either argument is NaN (it returned `0.0` or a
  quadrant angle).
- `Trig.tan` keeps its `+-1e308` pole sentinel, now on the new `sin` and `cos`.
  `sqrt` and `atan` are unchanged (checked against the vectors: 0 failing rows).

### Added

- `Trig.sqrtChecked(_:) throws` and `TrigError.negativeSquareRoot`: a non-trapping twin of
  `Trig.sqrt` that throws for exactly the inputs where `Trig.sqrt` traps. `Trig.sqrt`
  behaviour is unchanged. Added so the `expect = error` vector rows can be tested
  (a `preconditionFailure` cannot be caught by XCTest; survey PHY00a 8.1.5).

### Tests

- `ConformanceTests`: decodes every row of `trig-reference-v1` (62,154 rows, Part II
  included) by `Double(bitPattern:)`, calls the public API and requires identical bits (any
  NaN matches a NaN result), then checks accuracy against `cr` (sin/cos 3 steps,
  radians/degrees 1; `tan` named as a no-claim op; an unknown op, or another op with a `cr`
  and no limit, fails).
- `ConstantsTests`: asserts the bits of every constant against the fixture manifest and
  against the spec table, the decimal literals the source still writes, and the PHY00a 3.3
  special values.
- Verified in a `-c release` build, and in a release build with `-target-cpu haswell`
  (FMA available): same results, and no FMA instruction in the generated code.

## Earlier, unreleased changes (now part of 0.2.0)

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

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of Swift trig package.
- `Trig.sin(_:)` — sine via 20-term Maclaurin series with range reduction.
- `Trig.cos(_:)` — cosine via 20-term Maclaurin series with range reduction.
- `Trig.tan(_:)` — tangent as sin/cos ratio with pole guard.
- `Trig.sqrt(_:)` — square root via Newton's (Babylonian) iterative method.
- `Trig.atan(_:)` — arctangent via Taylor series with half-angle reduction.
- `Trig.atan2(_:_:)` — four-quadrant arctangent.
- `Trig.radians(_:)` — degrees to radians conversion.
- `Trig.degrees(_:)` — radians to degrees conversion.
- `PI` constant to full Double precision.
- Comprehensive XCTest suite covering all functions.
- `Package.swift`, `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`, `required_capabilities.json`.
- No external dependencies — pure Swift stdlib only.
