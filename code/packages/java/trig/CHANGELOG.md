# Changelog — trig (Java)

## [0.2.0] — 2026-11-16

Converged on the shared reference algorithm TRIG-REF-1 of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (slice TC-7). `sin`, `cos`,
`tan`, `radians` and `degrees` now return the same 64 bits as every other
converged port, checked against all 62,154 vectors of
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md).

### Changed (behaviour)

- **`sin`, `cos`: new algorithm, result bits change for almost every input.**
  Nearest-integer Cody-Waite range reduction (`k = (x * 2/pi + MAGIC) - MAGIC`,
  `pi/2` held as five pieces, four of 24 bits so `k * piece` is exact), then a
  fixed ten-term Horner polynomial on stored coefficients, then quadrant
  selection with `((int) k) & 3`. There is no division and no `Math` call. This
  replaces `x % TWO_PI` (which drifted by `k * 2.4e-16` rad: 3.9e-11 at 1e6) and
  the 20-term recurrence. Accuracy is within 3 ulp of the exact value (observed
  worst 2.38 ulp) over the whole domain.
- **`sin`, `cos`, `tan` return NaN for `|x| > 2^29` (536870912), for NaN and for
  the infinities.** Previously they returned a finite number that grew less
  accurate with `|x|`. Callers that accumulate a phase must reduce it first.
- **`sin(-0.0)` is now `-0.0`** (was `+0.0`); `|x| < 2^-27` returns `x` exactly
  (`sin`) or `1.0` (`cos`).
- `tan` is `sin / cos` of the new functions with the unchanged `+-1e308` pole
  sentinel; it now inherits the NaN domain. No accuracy claim is made for `tan`.
- **`radians` / `degrees`** multiply by the bit-pinned constants `pi/180`
  (`0x3F91DF46A2529D39`) and `180/pi` (`0x404CA5DC1A63C1F8`): the same values
  `deg * (PI / 180.0)` produced, so their bits do not change; the constants are
  now explicit and asserted.
- **`atan2` returns NaN when either argument is NaN** (it returned `0.0` for a
  NaN `x`).
- `TWO_PI` and `HALF_PI` are package-private (test access) instead of private.
- Version `0.1.0` to `0.2.0` in `build.gradle.kts`.

### Unchanged

- `sqrt`, `atan` and `atan2` (apart from the NaN guard) keep their pinned
  Part II algorithm, and the vectors confirm them bit for bit.

### Tests

- Add `ConformanceTest`: decodes every `x` with `Double.longBitsToDouble`,
  compares the result bits as integers (NaN matches any NaN) over the four
  fixture groups, checks accuracy against the `cr` column (sin/cos 3 steps,
  radians/degrees 1 step; `tan` carries no claim; any other op with a `cr` and an
  unknown op fail), asserts every constant's bits against the fixture manifest,
  the fixture's own hashes, and the named contract points of PHY00a 3.3.
- Run against the unchanged 0.1.0 port the new test failed on: sin 9,257 rows,
  cos 8,298, tan 9,677, atan2 6 (of 12,408 rows per trig op); `radians`,
  `degrees`, `sqrt` and `atan` had no failing row.

## Carried into 0.2.0 (previously Unreleased)

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

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of trigonometric functions from first principles.
- `Trig.sin(x)` — 20-term Maclaurin series with range reduction to [-π, π].
- `Trig.cos(x)` — 20-term Maclaurin series with range reduction to [-π, π].
- `Trig.tan(x)` — implemented as `sin(x)/cos(x)` with pole guard.
- `Trig.sqrt(x)` — Newton's (Babylonian) method; throws `ArithmeticException` for negative input.
- `Trig.atan(x)` — Taylor series with two-layer range reduction (|x|>1 and half-angle).
- `Trig.atan2(y, x)` — four-quadrant arctangent with correct quadrant handling.
- `Trig.radians(deg)` / `Trig.degrees(rad)` — angle unit conversion.
- `Trig.PI` — π constant to full double precision.
- 57 unit tests covering special values, symmetry identities, Pythagorean identity,
  large-input range reduction, roundtrip conversions, and all four atan2 quadrants.
