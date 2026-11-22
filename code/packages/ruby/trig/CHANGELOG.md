# Changelog

## [0.3.0] - 2026-11-16

Converges this port on the TRIG-REF-1 reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (TRIG-CONV step TC-13).
`sin`, `cos`, `tan`, `radians` and `degrees` now return exactly the same bits as
every other converged port, checked bit for bit against
`code/specs/fixtures/trig-reference-v1` (62,154 vectors). This is the minor-version
bump that also absorbs the unreleased changes below (the previous release
recorded here is 0.2.0 and no manifest existed).

### Changed (numeric behaviour)

- `sin` and `cos`: nearest-integer Cody-Waite range reduction (`k = (x*2/pi + MAGIC) - MAGIC`,
  five-slice `pi/2`), reducing to `|r| <= pi/4` instead of a floored `%` by `2*PI`
  to `[-PI, PI]`; ten-term Horner kernels on stored coefficients instead of a
  20-term recurrence with a division per term. Results differ from 0.2.0 in the
  last bits almost everywhere; `sin` and `cos` are within 3 ulp of the correctly
  rounded value and `radians`/`degrees` within 1 ulp (previously the reduction error
  alone reached many ulps near multiples of pi).
- `sin(-0.0)` is now `-0.0` (was `+0.0`); `|x| < 2^-27` returns `x` for `sin` and `1.0` for `cos`.
- **Domain restriction:** `sin`, `cos` and `tan` return NaN for NaN, infinities and
  `|x| > 2^29` (previously a finite, increasingly inaccurate value, or an
  exception for infinity). No finite wrong answer is returned.
- `radians(d)` is `d * (PI/180)` and `degrees(r)` is `r * (180/PI)`: one multiply by a
  bit-pinned constant instead of two roundings (`d * PI / 180`).
- `tan` is `sin / cos` of the new kernels with the unchanged `+-1e308` pole sentinel; it
  is NaN outside the domain. No accuracy claim is made for `tan`.
- `atan2` returns NaN when either argument is NaN (previously `0.0`).
- `Integer` arguments are converted once with `to_f` at entry.
- `PI`, `TWO_PI` and `HALF_PI` are pinned by bit pattern; `INV_PIO2`, `PIO2`, `MAGIC`,
  `LIMIT`, `TINY`, `S`, `C`, `RAD_PER_DEG`, `DEG_PER_RAD` are new public constants.
- `range_reduce` is removed (private helper; replaced by the private `reduce`).

### Added

- `coding_adventures_trig.gemspec` and `lib/trig/version.rb` (`Trig::VERSION`): the
  package had no gem manifest and was not publishable (repo rule 10).
- `test/test_conformance.rb`: decodes every `trig-reference-v1` row by bit cast and
  compares result bits as integers; asserts the fixture hashes, every embedded constant,
  the decimal literals' bits, accuracy against the `cr` column (sin/cos 3 ulp,
  radians/degrees 1 ulp, `tan` named as a no-claim op, any other op with a `cr` and no
  limit fails, unknown ops fail), and the PHY00a 3.3 special values.
- `BUILD` and `BUILD_windows` run the conformance test.

## Changes absorbed into 0.3.0 (previously Unreleased)

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

- `Trig.sqrt(x)` — square root via Newton's method; raises `ArgumentError` for negative inputs.
- `Trig.tan(x)` — tangent as sin/cos ratio with pole guard.
- `Trig.atan(x)` — arctangent via Taylor series with outer and half-angle range reduction.
- `Trig.atan2(y, x)` — four-quadrant arctangent.
- `HALF_PI` public constant.
- `atan_core(x)` private class method helper.
- Tests for all new functions in `test/test_trig.rb`.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `Trig.sin(x)` via Maclaurin series with range reduction
- `Trig.cos(x)` via Maclaurin series with range reduction
- `Trig.radians(deg)` degree-to-radian conversion
- `Trig.degrees(rad)` radian-to-degree conversion
