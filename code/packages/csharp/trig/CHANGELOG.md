# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-11-16

Converges the package on the PHY00a reference algorithm TRIG-REF-1
(TC-9 of TRIG-CONV), so that `Sin`, `Cos`, `Tan`, `Radians` and `Degrees`
return the same bits as every other converged `trig` port.

### Changed (behaviour)

- `Sin` and `Cos` use nearest-integer Cody-Waite range reduction (five pi/2
  pieces, `MAGIC` rounding) and ten-term Horner kernels on stored coefficients,
  replacing `x % 2pi` with a clamp and the 20-term recurrence. Every result may
  differ in the last bits; the new results are within 3 ulp of the correctly
  rounded value (observed worst 2 representable steps), and large arguments
  are now accurate where they were wrong by about `k * 2.4e-16` radians.
- **`Sin`, `Cos` and `Tan` return NaN for `|x| > 2^29`** (about 5.4e8), for
  NaN and for both infinities. Callers that accumulate a phase must reduce it
  themselves.
- `Sin(-0.0)` is now `-0.0` (it was `+0.0`); `|x| < 2^-27` returns `x` (`Sin`)
  or `1` (`Cos`) exactly.
- `Radians` and `Degrees` multiply by the pinned constants `RAD_PER_DEG` and
  `DEG_PER_RAD` (bit-identical to the old `PI / 180.0` and `180.0 / PI`;
  no value change).
- `Tan` follows the new `Sin` and `Cos`; the `+-1e308` pole sentinel is
  unchanged and `Tan` has no accuracy claim (PHY00a 3.8).
- `Atan2` returns NaN when either argument is NaN (it returned `0.0` for a NaN
  `x`). The pinned conventions (`Atan2(-0, -1) = +PI`, `Atan2(0, 0) = 0`) are
  unchanged.
- `PI` stays a `const`; the other constants are built from IEEE bit patterns
  with `BitConverter.Int64BitsToDouble`. `Math.Sin`, `Math.Cos`, `%` and
  `Math.FusedMultiplyAdd` are not used.
- Package version 0.1.0 to 0.2.0.

### Tests

- Add `ConformanceTests`: all 62,154 rows of
  `code/specs/fixtures/trig-reference-v1` bit for bit, fixture integrity
  (manifest hashes and row counts), accuracy against the `cr` column (`sin`,
  `cos` within 3 steps, `radians`, `degrees` within 1; `tan` named as the one op
  with no claim; any other op with a `cr` but no limit fails; an unknown op
  fails) and the PHY00a 3.3 contract points.
- Add `ConstantsTests`: every constant's bits against the manifest.
- Against the unchanged 0.1.0 code the new test failed on `sin` 9257,
  `cos` 8298, `tan` 9677 and `atan2` 6 rows.

### Dependents

- `affine2d`, `arc2d`, `wave` and `bezier2d` tests pass unchanged.

### Also in this release (previously Unreleased)

#### Changed

- Return `Atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

#### Fixed

- Preserve negative zero and both signs of the minimum subnormal in `Atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

#### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

## [0.1.0] - 2026-04-15

### Added

- First-principles `Trig` implementation covering `sin`, `cos`, `tan`,
  `sqrt`, `atan`, `atan2`, `Radians`, and `Degrees`
- Literate inline documentation explaining Maclaurin series, range reduction,
  and Newton iteration
- Unit tests covering canonical identities, quadrant handling, and conversion
  round-trips
