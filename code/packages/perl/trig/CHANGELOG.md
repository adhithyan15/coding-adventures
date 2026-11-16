# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-11-16

Converges this port on the TRIG-REF-1 reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) (TRIG-CONV step TC-15).
`sin_approx`, `cos_approx`, `tan_approx`, `degrees_to_radians` and
`radians_to_degrees` now return exactly the same bits as every other converged
port, checked bit for bit against `code/specs/fixtures/trig-reference-v1`
(62,154 vectors). This is the minor-version bump that also absorbs the
unreleased changes below. The previous release recorded here is 0.2.0 while
`$VERSION` still read `0.01`; `$VERSION` is now `0.3.0`, in step with this file.

### Changed (numeric behaviour)

- `sin_approx` and `cos_approx`: nearest-integer Cody-Waite range reduction
  (`k = (x*2/pi + MAGIC) - MAGIC`, five-slice `pi/2`), reducing to `|r| <= pi/4`
  instead of `x - 2*pi*int(x/2pi)` clamped to `[-pi, pi]`; ten-term Horner kernels
  on stored coefficients instead of a 21-term recurrence with a division per term.
  Results differ from 0.2.0 in the last bits almost everywhere; sin and cos are
  within 3 ulp of the correctly rounded value and the conversions within 1 ulp.
- `sin_approx(-0.0)` is now `-0.0` (was `+0.0`); `|x| < 2^-27` returns `x` for sin
  and `1.0` for cos. (Perl has no negative-zero integer, so the integer literals
  `0` and `-0` still give `+0`; only an NV `-0.0` carries a sign.)
- **Domain restriction:** `sin_approx`, `cos_approx` and `tan_approx` return NaN for
  NaN, infinities and `|x| > 2^29` (previously a finite, increasingly wrong value:
  `sin_approx(1e20)` was `0` and `cos_approx(1e20)` was `1`).
- `degrees_to_radians($d)` is `$d * (PI/180)` and `radians_to_degrees($r)` is
  `$r * (180/PI)`: one multiply by a bit-pinned constant instead of two roundings.
  `radians_to_degrees(-0.0)` is now `-0.0` (was `+0.0`, because `* 180.0` took
  Perl's integer path).
- `tan_approx` gains the pole sentinel every other port already had: `+-1e308`
  (signed like the sine) when `|cos| < 1e-15`; it is NaN outside the domain. No
  accuracy claim is made for `tan`. `CodingAdventures::Affine2D` `skew_x` / `skew_y`
  call `tan_approx` and now see the sentinel at 90 degrees instead of a huge
  quotient.
- `atan2_approx` returns NaN when either argument is NaN (previously `0.0`).
- `sin_deg`, `cos_deg`, `tan_deg` are unchanged in definition; they go through the
  new `degrees_to_radians`.
- `$PI`, `$TWO_PI` and `$HALF_PI` are pinned by bit pattern; `$INV_PIO2`, `@PIO2`,
  `$MAGIC`, `$LIMIT`, `$TINY`, `@S`, `@C`, `$RAD_PER_DEG`, `$DEG_PER_RAD`, `$NAN` and
  the legacy thresholds (`$TAN_POLE`, `$TAN_BIG`, `$ATAN_TERM`, `$SQRT_REL`,
  `$SQRT_TINY`, `$MAX_FINITE`) are new package variables, all built from bits.
- `_range_reduce` is replaced by the private `_reduce`.
- The module dies at load on a Perl whose NV is not an 8-byte double.

### Added

- `t/02-conformance.t`: decodes every `trig-reference-v1` row by bit cast and
  compares result bits (never printed numbers); asserts the fixture hashes and row
  counts, every embedded constant against the manifest, accuracy against the `cr`
  column (sin/cos 3 ulp, radians/degrees 1 ulp, `tan` named as a no-claim op, any
  other op with a `cr` and no limit fails, unknown ops fail), and the PHY00a 3.3
  special values including `sin(-0.0)` for NV and integer arguments. `BUILD` and
  `BUILD_windows` run it through `prove ... t/`.
- `required_capabilities.json` (no capabilities) and the `JSON::PP` / `Digest::SHA`
  test dependencies.

### Tests

- Before this change the conformance test failed 9056 sin, 8349 cos, 9652 tan,
  3382 radians, 2471 degrees and 6 atan2 rows (of 12,408 each, 54 atan2); sqrt
  and atan already matched. After it, no row fails.

## Changes absorbed into 0.3.0 (previously Unreleased)

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.
- Return `atan($x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
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

- `sqrt_approx($x)` — square root via Newton's method; `die`s for negative inputs. Exported.
- `atan_approx($x)` — arctangent via Taylor series with outer and half-angle range reduction. Exported.
- `atan2_approx($y, $x)` — four-quadrant arctangent. Exported.
- `$HALF_PI` package variable (π/2).
- `_atan_core($x)` private helper.
- All new functions added to `@EXPORT_OK`.
- Tests for all new functions in `t/01-basic.t`.

## [0.1.0] - 2026-03-28

### Added

- Initial package scaffolding generated by scaffold-generator
