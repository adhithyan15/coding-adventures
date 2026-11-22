# Changelog

## [Unreleased]

## [0.3.0] - 2026-11-16

Converges this port on the TRIG-REF-1 reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (TRIG-CONV slice TC-14). `sin`, `cos`,
`radians` and `degrees` now return exactly the bits of every other converged port
(Python, Rust, TypeScript, Ruby, ...), checked against the 62,154 shared vectors of
`code/specs/fixtures/trig-reference-v1` by `tests/test_conformance.lua`. The manifest
(rockspec `0.1.0-1`, `trig.VERSION = "0.1.0"`) had lagged the changelog's 0.2.0; it
now says 0.3.0 and the rockspec is renamed `coding-adventures-trig-0.3.0-1.rockspec`.

### Changed (numeric behaviour: read this before upgrading)

- **`sin`, `cos`: new algorithm.** Nearest-integer Cody-Waite range reduction (`k` rounded
  with the `MAGIC` add-and-subtract, `pi/2` carried as five pieces to 149 bits) replaces
  `x - TWO_PI*math.modf(x/TWO_PI)`; ten-term Horner kernels on stored coefficients
  replace the 21-term division recurrence. Results differ from 0.2.0 in the last bits
  everywhere (worst error now 2.38 ulp, gated at 3), and differ materially for
  `|x| > 1e5`, where the old reduction subtracted multiples of a 53-bit `2*pi`.
- **`sin`, `cos`, `tan` return NaN for `|x| > 2^29` (about 5.4e8), for infinities and for
  NaN.** They used to return a finite, meaningless number (`sin(1e20)` was `0`). Callers
  that accumulate a phase must reduce it to a fractional cycle first (see `lua/wave` 0.2.0).
- **`sin(-0.0)` is now `-0.0`** (it was `+0.0`); `|x| < 2^-27` returns `x` exactly for
  `sin` and `1.0` for `cos`.
- **`tan` gains the pole sentinel:** `|cos(x)| < 1e-15` returns `+1e308` or `-1e308` by the
  sign of `sin(x)`, as every other port does. `tan(HALF_PI)` was about `1.6e16`. This
  reaches `affine2d` `skew_x`/`skew_y` (no `affine2d` test pinned the old value).
- **`radians` and `degrees` are one multiply by a pinned constant** (`RAD_PER_DEG`,
  `DEG_PER_RAD`) instead of `deg * PI / 180.0`; 0.65 and 0.81 ulp worst case, down
  from 1.5.
- **`atan2` of a NaN argument is NaN** (it used to fall through to `0.0`).
- Integer arguments are converted to floats once at entry (`x * 1.0`, which keeps `-0.0`);
  `sin`, `cos`, `tan`, `sqrt` always return floats, so `sqrt(0)` is `0.0`, not `0`.
- `PI`, `TWO_PI` and `HALF_PI` are built from their bit patterns. `sqrt`, `atan` and
  `atan2` are otherwise unchanged (PHY00a 3.9 pins them).

### Added

- Exported reference constants: `INV_PIO2`, `PIO2` (five pieces), `MAGIC`, `LIMIT`, `TINY`,
  `S` (sine coefficients, `S[1]..S[9]`), `C` (cosine, `C[2]..C[9]`), `RAD_PER_DEG`,
  `DEG_PER_RAD`.
- `tests/test_conformance.lua`: every vector of `trig-reference-v1`, bit for bit (NaN as a
  class); the manifest's constants asserted by bits; decimal literals asserted by bits;
  accuracy against the `cr` column (`sin`/`cos` 3 ulp, `radians`/`degrees` 1 ulp, `tan`
  named as a no-claim op, any other op with a `cr` and no limit fails, unknown ops fail).
  Against the unchanged 0.2.0 code 32,915 of the 62,154 rows failed: `sin` 9056,
  `cos` 8349, `tan` 9652, `radians` 3382, `degrees` 2470 (each of 12,408 rows) and `atan2`
  6 of 54; `atan` and `sqrt` already matched.

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

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-03

### Added

- `trig.sqrt(x)` — square root via Newton's method; `error()`s for negative inputs.
- `trig.atan(x)` — arctangent via Taylor series with outer and half-angle range reduction.
- `trig.atan2(y, x)` — four-quadrant arctangent.
- `trig.HALF_PI` public constant (π/2).
- `atan_core(x)` local (private) helper function.
- Tests for all new functions in `tests/test_trig.lua`.

## [0.1.0] - 2026-03-23

### Added

- `sin(x)` -- sine via Maclaurin series with 20-term convergence
- `cos(x)` -- cosine via Maclaurin series with 20-term convergence
- `tan(x)` -- tangent defined as sin/cos
- `radians(deg)` -- degrees-to-radians conversion
- `degrees(rad)` -- radians-to-degrees conversion
- `PI` and `TWO_PI` constants
- Internal range reduction to [-pi, pi] for numerical stability
- Literate programming style with inline explanations of the mathematics
- Comprehensive busted test suite (64 tests) covering landmark values, symmetry properties, Pythagorean identity, double-angle identities, complementary-angle identities, cross-validation against math library, and range reduction stress tests
