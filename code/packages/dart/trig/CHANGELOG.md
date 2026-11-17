# Changelog

## [0.2.0] - 2026-11-16

Converges the package on the PHY00a reference algorithm, TRIG-REF-1 (TC-16),
bit for bit against the shared vectors in
`code/specs/fixtures/trig-reference-v1`.  Before this change 9,760 `sin`, 8,323
`cos`, 10,167 `tan`, 3,382 `radians`, 2,470 `degrees` and 6 `atan2` of the
62,154 vectors disagreed; now none do.

### Changed (behaviour)

- `sin` and `cos`: Cody-Waite range reduction (`2/pi` quarter-turn count by the
  `1.5 * 2^52` trick, `pi/2` as five exact pieces) and Horner kernels replace
  `x % twoPi` and the 20-term running series.  Within 3 ulp of the correctly
  rounded value (worst measured 2.38 ulp); results differ from 0.1.0 in the last
  bits and by more for large angles.
- `sin`, `cos` and `tan` return NaN for `|x| > 2^29`, NaN and infinities (0.1.0
  folded large angles with `%` and returned a number).
- `sin(-0.0)` is now `-0.0`, and `|x| < 2^-27` returns `x` (`cos` returns 1).
- `radians` and `degrees` are a single multiply by a pre-divided bit-pattern
  constant (`pi/180`, `180/pi`), not `x * pi / 180`; within 1 ulp.
- `atan2` returns NaN when either argument is NaN (it returned `+-pi/2` for a NaN
  `x` with nonzero `y`).
- `atan` multiplies each series term left to right, `((term * -t^2) * (2n-1)) /
  (2n+1)`, instead of by the pre-rounded ratio; the last bit differs on about
  0.2-0.7% of inputs.
- `tan` keeps its `+-1e308` pole sentinel (a convention) and now makes no accuracy
  claim; it follows `sin` and `cos`.  `sqrt` is unchanged.
- Constants are built from bit patterns through `ByteData` (two 32-bit halves).

### Added

- `test/trig_reference_test.dart`: every vector checked as integer bits (VM only,
  `@TestOn('vm')`), accuracy against `cr` (sin/cos 3 ulp, radians/degrees 1 ulp,
  `tan` explicitly unclaimed, any other op with a `cr` and no limit fails), the
  constant table against the manifest, and a direct test of the `atan` grouping.
- `test/generated/trig_reference_vectors.dart`: the vectors as generated source
  (the package has no filesystem capability), produced by
  `code/scripts/generate_trig_reference_dart_fixtures.py`, decoded by the
  validated loader `test/support/vector_loader.dart`.

### Notes

- The bit-exactness claim is for the **Dart VM** (JIT and AOT; the suite also
  passes with `dart test --compiler exe`).  It is not claimed for dart2js.

## Unreleased

### Changed

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving negative zero and both signs of the minimum subnormal exactly.
- Consume the versioned language-neutral PHY00 corpus in package tests,
  including poles, quadrants, signed zero, subnormal, maximum finite,
  infinity, NaN, and validation cases.
- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.

## [0.1.0] - 2026-07-31

### Added

- Initial pure Dart implementation of the PHY00 trigonometry contract.
- First-principles sine, cosine, tangent, square root, arctangent, and
  four-quadrant arctangent implementations.
- Cross-language special-angle, identity, range-reduction, conversion, and
  inverse-function tests.
- Full-range Newton scaling with tiny-normal, subnormal, infinity, NaN, and
  signed-zero boundary coverage.
- Schema-v1 empty capability metadata and portable BUILD entry points.
