# Changelog

## [0.2.0] - 2026-11-16

Converges this port on the TRIG-REF-1 reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (TRIG-CONV, slice TC-10). It is
a numeric change: **the last bits of every `sin`, `cos` and `tan` result change**,
and arguments beyond 2^29 change materially. `radians` and `degrees` were already
the reference form and keep their bits.

### Changed

- The library assembly is now `CodingAdventures.Trig.FSharp` and the test
  assembly `CodingAdventures.Trig.FSharp.Tests` (both matched their PackageId
  only by accident before). `csharp/trig` emits `CodingAdventures.Trig`, and two
  packages with one assembly name made coverlet report a flat 0% in CI.

- `sin` and `cos`: nearest-integer Cody-Waite range reduction (`k` by the
  add-and-subtract-`MAGIC` trick, `r = x - k*pi/2` from five pieces, the first
  four with exact products), then ten-term Horner kernels on stored coefficients
  with a quadrant switch on `int k &&& 3`. No `%`, no division, no host library, no
  `Math.FusedMultiplyAdd`. Accurate to 3 ulp of the exact value on `|x| <= 2^29`.
  The old `x % 2pi` reduction and the 20-term recurrence (`term * -z / denom`) are gone.
- **`sin`, `cos` and `tan` return NaN for `|x| > 2^29`, NaN and the infinities**
  (they used to return a number, an increasingly wrong one). Callers that
  accumulate an angle must reduce it first; `wave` already reduces time and phase.
- `sin(-0.0)` is now `-0.0` (it was `+0.0`); `0 < |x| < 2^-27` returns `x`.
- `tan` is `sin / cos` with the existing `+-1e308` pole sentinel (a pinned legacy
  contract), now fed by the accurate `sin` and `cos`.
- `atan2` returns NaN when either argument is NaN (it used to return `0` or
  `+-pi/2`).
- `atan` really stops its series at the first term below `1e-17`, as the reference
  specifies (the old `if ... then ()` was a no-op and all 30 terms were summed; the
  two are bit-identical).
- Every constant of the algorithm lives in the new public module `TrigConstants`,
  built from its pinned bit pattern with `BitConverter.Int64BitsToDouble`.

### Tests

- Add `ConformanceTests`: every row of the four shared `trig-reference-v1` groups
  (62,154 vectors) decoded by bit cast and compared bit for bit; accuracy against
  the correctly rounded column (sin and cos 3 ulp, radians and degrees 1 ulp, `tan`
  named as the one no-claim op, any other op with a `cr` value, or an unknown op,
  failing); and the contract cases of PHY00a 3.3 and 3.8.
- Add `ConstantsTests`: the library's constants equal the manifest's, bit for bit.

## [Unreleased]

### Changed

- Return `atan x` unchanged for `|x| <= 2^-27` before half-angle reduction,
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

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-15

### Added

- First-principles `Trig` module implementing `sin`, `cos`, `tan`, `sqrt`,
  `atan`, `atan2`, `radians`, and `degrees`
- Literate comments covering Maclaurin series, range reduction, and Newton's
  method
- Test coverage for standard identities, large inputs, and quadrant handling
