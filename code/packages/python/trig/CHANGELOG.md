# Changelog

## [0.3.0] - 2026-11-16

Converges this port on the TRIG-REF-1 reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (TRIG-CONV, slice TC-1). It is
a numeric change: **the last bits of every `sin`, `cos`, `tan`, `radians` and
`degrees` result change**, and large arguments change materially. The documented
contract (a sine to 1e-10 on [-1e6, 1e6]) is unchanged, so this is a minor
version (owner decision OQ-12).

### Changed

- `sin` and `cos`: nearest-integer Cody-Waite range reduction (`k` by the
  add-and-subtract-`MAGIC` trick, `r = x - k*pi/2` from five exact-product
  pieces), then ten-term Horner kernels on stored coefficients with a quadrant
  switch on `int(k) & 3`. No division, no `%`, no host library. Accurate to
  3 ulp of the exact value on `|x| <= 2**29` (worst case measured against exact
  rational arithmetic: 2.38 ulp for `sin`, 2.37 for `cos`). Constants are built
  from bit patterns.
- **`sin`, `cos` and `tan` return NaN for `|x| > 2**29`, NaN and the infinities.**
  They used to return a number (an increasingly wrong one, and `0` or an
  arbitrary value for `1e20`). Callers that accumulate an angle must reduce it
  first; `oscillator` does (its 0.1.2).
- `sin(-0.0)` is now `-0.0` (it was `+0.0`); `0 < |x| < 2**-27` returns `x`.
- `radians` and `degrees` are one multiply by a pre-divided, bit-pinned constant
  (`pi/180`, `180/pi`) instead of `deg * PI / 180`: one rounding, no overflowing
  intermediate, 0.65 and 0.80 ulp worst case.
- `tan` is `sin / cos` with the existing `+-1e308` pole sentinel, a pinned
  legacy contract (not an approximation), now fed by the accurate `sin`/`cos`.
- `atan` runs the series term strictly left to right, as the reference pins.
- `atan2` returns NaN when either argument is NaN (it used to return `0.0`).
- Integer arguments are converted to `float` once at entry; `HALF_PI` and
  `TWO_PI` are pinned by bit pattern.

### Fixed

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.
- Preserve negative zero and both signs of the minimum subnormal in `atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error in `sqrt`.

### Removed

- The 20-term recurrence series, the `%`-based `_range_reduce` helper and the
  `[-pi, pi]` clamp.

### Tests

- `tests/test_conformance.py` decodes every row of
  [`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md)
  (61,854 vectors: `sin`, `cos`, `tan`, `radians`, `degrees`, `sqrt`, `atan`,
  `atan2`) by a bit cast and requires the result bits to match **exactly**
  (NaN as a class, `error` as `ValueError`), asserts every embedded constant's
  bits, the domain guard, signed zero and the pinned `atan2` conventions, and
  checks the accuracy against the correctly rounded column.
- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
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
