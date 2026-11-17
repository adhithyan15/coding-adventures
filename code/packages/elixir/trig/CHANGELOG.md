# Changelog

## [0.3.0] - 2026-11-16

Converges the Elixir port on the PHY00a reference algorithm TRIG-REF-1 (TC-17).
The `mix.exs` version moves from 0.1.0 to 0.3.0, closing the manifest/CHANGELOG
lag (the CHANGELOG already recorded 0.2.0).

### Changed (numeric behaviour)

- `sin`, `cos`: Cody-Waite range reduction (nearest-integer quotient by the
  `(t + MAGIC) - MAGIC` trick, five 24-bit pieces of pi/2) and ten-term Horner
  kernels on stored coefficients replace the 20-term recurrence and
  `Float.round`.  Results differ in the last bits everywhere and are now within
  3 ulp of the correctly rounded value; large `|x|` (up to 2^29) is accurate
  where it used to lose digits.  No division remains in `sin`/`cos`.
- **Behaviour change: `sin`, `cos` and `tan` raise `ArithmeticError` for
  `|x| > 2^29`.**  TRIG-REF-1 returns NaN there; a BEAM float cannot be NaN, so
  the port raises (PHY00a 3.3, 5, OQ-10).  Previously a finite, meaningless
  number was returned.  Callers that pass an unreduced user angle (for example
  through `arc2d`, `affine2d`, `point2d`, `wave`) now get an exception beyond
  about 5.4e8 radians.
- `sin(-0.0)` is `-0.0`; `|x| < 2^-27` returns `x` (`sin`) or `1.0` (`cos`).
- `radians` and `degrees` are one multiply by a stored constant
  (`RAD_PER_DEG`, `DEG_PER_RAD`) instead of `x * pi / 180` and `x * 180 / pi`
  (within 1 ulp).  `degrees(1.7e308)` raises `ArithmeticError` (the result
  would be infinite).  Integers are still accepted and converted with `* 1.0`.
- `tan` keeps its `+-1e308` pole sentinel, now on the new `sin` and `cos`.
- Added `Trig.constants/0`, a map of every PHY00a 3.2 constant by its spec name,
  so the tests can assert each one's bits against the shared manifest.
- Every constant is built from its bit pattern (`<<f::float-64>> = <<bits::64>>`).
- Part II (`sqrt`, `atan`, `atan2`) is bit-identical to before (checked against
  the vectors); only its thresholds now come from the pinned constants.

### Tests

- Added `test/conformance_test.exs`: every row of the four
  `trig-reference-v1` groups (62,154 vectors) bit for bit, accuracy against the
  correctly rounded column (sin/cos 3 ulp, radians/degrees 1 ulp; `tan` named as
  a no-claim op; an op with a `cr` value but no limit fails), the manifest
  hashes, and every constant's bits.
- Rows flagged `nonfinite=unrepresentable` (a NaN or infinity input that cannot
  exist as a BEAM float) are counted and skipped by operation (85 rows); rows
  flagged `nonfinite=rejects` or `beam-raises` require `ArithmeticError`.

### Changed (earlier, unreleased)

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize finite square-root inputs by powers of four before Newton iteration
  so the complete BEAM binary64 exponent range converges.
- Record IEEE-754 NaN and infinity fixture cases as not applicable because
  Erlang/OTP does not expose them as ordinary BEAM floats.

### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover tiny, minimum-subnormal, maximum-finite, signed-zero, and negative-input
  behavior from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

## [0.2.0] - 2026-04-03

### Added

- `Trig.sqrt/1` — square root via Newton's method; raises `ArithmeticError` for negative inputs.
- `Trig.tan/1` — tangent as sin/cos ratio with pole guard.
- `Trig.atan/1` — arctangent via Taylor series with outer and half-angle range reduction.
- `Trig.atan2/2` — four-quadrant arctangent.
- `@half_pi` private module attribute.
- `atan_core/1` private helper using `Enum.reduce_while` for early termination.
- `sqrt_iterate/3` private tail-recursive helper for Newton iterations.
- Tests for all new functions in `test/trig_test.exs`.

## [0.1.0] - 2026-03-22

### Added
- `pi()` constant to double-precision accuracy
- `sin(x)` via Maclaurin series with range reduction
- `cos(x)` via Maclaurin series with range reduction
- `radians(deg)` degree-to-radian conversion
- `degrees(rad)` radian-to-degree conversion
