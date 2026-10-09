# Changelog

## [Unreleased]

### Security

- Move the test toolchain from `jest` 29 to `jest` ^30.5.2, `@types/jest` to ^30.0.0, and `ts-jest` to ^29.4.14, which accepts jest 30. jest 29 reaches `braces` 3.0.3 through `micromatch`. `braces` has a high-severity stack-exhaustion advisory (GHSA-vfj7-8cjw-p6xm) and no patched release, so the only fix is the jest 30 tree, which no longer depends on it. A moderate `sprintf-js` advisory (GHSA-hp3w-g68c-fv3c) remains through `babel-plugin-istanbul` → `@istanbuljs/load-nyc-config` → `js-yaml` 3 → `argparse` 1. It has no upstream fix and is tracked in `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md`.

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
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

## [0.2.0] - 2026-04-03

### Added

- `sqrt(x)` — square root via Newton's (Babylonian) iterative method; throws for negative inputs.
- `tan(x)` — tangent as sin/cos ratio with pole guard.
- `atan(x)` — arctangent via Taylor series with outer and half-angle range reduction.
- `atan2(y, x)` — four-quadrant arctangent.
- Tests for all new functions covering landmark values, roundtrips, and edge cases.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `sin(x)` via Maclaurin series with range reduction
- `cos(x)` via Maclaurin series with range reduction
- `radians(deg)` degree-to-radian conversion
- `degrees(rad)` radian-to-degree conversion
