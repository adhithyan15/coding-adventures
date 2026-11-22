# Changelog

## 0.2.0 - 2026-11-16

Converges this port on the TRIG-REF-1 reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (TRIG-CONV, slice TC-12). It is
a numeric change: **the last bits of every `sin`, `cos`, `tan`, `radians` and
`degrees` result change**, and large arguments change materially. The minor bump
also moves the upper bound that `affine2d`, `arc2d`, `point2d` and `wave` declared,
which those packages widen to `<0.3` in the same change.

### Changed

- `sin` and `cos`: nearest-integer Cody-Waite range reduction (`k` by the
  add-and-subtract-`MAGIC` trick, `r = x - k*pi/2` from five exact-product pieces),
  then ten-term Horner kernels on stored coefficients with a quadrant switch on
  `truncate k .&. 3`. No `mod'`, no `floor`, no division, no host library. Accurate to
  3 ulp of the exact value on `|x| <= 2^29` (2.38 ulp worst case measured). The old
  `mod'` reduction floored the exact rational quotient and left the reduced angle in
  `[0, pi]` or `(-pi, 0)`.
- **`sin`, `cos` and `tan` return NaN for `|x| > 2^29`, NaN and the infinities**
  (they used to return a number, an increasingly wrong one). Callers that accumulate
  an angle must reduce it first; `wave` already reduces time and phase.
- `sin (-0.0)` is now `-0.0` (it was `+0.0`); `0 < |x| < 2^-27` returns `x`.
- `radians` and `degrees` are one multiply by a pre-divided, bit-pinned constant
  (`pi/180`, `180/pi`) instead of `value * pi / 180`: one rounding, no overflowing
  intermediate.
- `tan` is `sin / cos` with the existing `+-1e308` pole sentinel (a pinned legacy
  contract), now fed by the accurate `sin` and `cos`.
- `atan2` returns NaN when either argument is NaN (a NaN `x` with a nonzero `y`
  used to return `+-pi/2`).
- `atan` stops its series at the first term below `1e-17`, as the reference
  specifies (it used to sum all 30 terms; the two are bit-identical).
- `piValue`, `twoPi` and `halfPi` are built from their bit patterns. Every constant
  of the algorithm lives in the new exposed module `Trig.Constants`.

### Tests

- Add `ConformanceSpec`: every row of the four shared `trig-reference-v1` groups
  (62,154 vectors) decoded by bit cast and compared bit for bit, the manifest
  constants compared with the module's, the accuracy limits against the correctly
  rounded column (sin and cos 3 ulp, radians and degrees 1 ulp, `tan` named as a
  no-claim op, any other op with a `cr` value failing), and the contract cases of
  PHY00a 3.3 and 3.8.

## Previously unreleased, released in 0.2.0

### Changed

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
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

## 0.1.0 - 2026-07-18

- Add first-principles sine and cosine using range-reduced Maclaurin series.
- Add degree/radian conversion and Newton-method square root.
- Add tangent with pole guards plus arctangent and four-quadrant arctangent.
