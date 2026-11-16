# Changelog

## [Unreleased]

## [0.2.0] - 2026-11-16

### Changed

- **Numeric change (TRIG-CONV TC-4, PHY00a):** `trig_sin`, `trig_cos`, `trig_radians`
  and `trig_degrees` follow the reference algorithm TRIG-REF-1 and match every row of the
  shared vectors in `specs/fixtures/trig-reference-v1` bit for bit: nearest-integer
  Cody-Waite reduction (`MAGIC` add-and-subtract, five pieces of pi/2), ten-term Horner
  kernels on stored hex-float coefficients, no division, quadrant by `(int)k & 3`. The
  last bits of every `sin` and `cos` change and large arguments change materially (the old
  `long long` truncation left residue: `sin(1e20)` was `0` and `cos(1e20)` was `1`).
  Minor version bump (owner decision OQ-12). The old `d_trunc`, `d_fmod` and
  `range_reduce` helpers are gone.
- **`trig_sin`, `trig_cos` and `trig_tan` return NaN for NaN, the infinities and
  `|x| > 2^29`** (previously a finite number for every finite argument, wrong beyond about
  1e6). Callers that accumulate a phase must reduce it themselves.
- `trig_sin(-0.0)` is now `-0.0` (was `+0.0`); `|x| < 2^-27` returns `x` for sin and `1`
  for cos.
- `trig_tan`: same `+-1e308` pole sentinel, now computed from the new `sin` and `cos`
  (last-bit changes everywhere; NaN beyond 2^29). No other `tan` edge changed.
- `trig_atan2` returns NaN when either argument is NaN (it returned `0.0`; 6 vectors).
  The other `atan2` conventions are pinned unchanged: `atan2(-0, x < 0) = +PI`,
  `atan2(0, 0) = 0`, `atan2(+-inf, +-inf) = NaN`.
- `trig_radians` / `trig_degrees` multiply by the bit-pinned constants `pi/180` and
  `180/pi` (the same value the old `PI / 180.0` folded to; no result changes).
- Part II against the vectors: `trig_sqrt` and `trig_atan` already matched all 22 and 38
  rows; only the six NaN `atan2` rows differed.
- Arithmetic guards: contraction is switched off in the source per compiler (clang
  `#pragma STDC FP_CONTRACT OFF`, GCC `#pragma GCC optimize("fp-contract=off")`, MSVC
  `#pragma fp_contract(off)`); the build refuses `-ffast-math` and `FLT_EVAL_METHOD != 0`.

### Tests

- `tests/conformance_test.c` decodes all 62,154 vectors by a bit cast and compares result
  bits (NaN as a class, `error` as `TRIG_ERR_DOMAIN`), checks accuracy against the `cr`
  column (sin/cos within 3 steps, radians/degrees within 1; `tan` is a named no-claim op
  and any other op with a `cr` value but no limit fails), asserts the bits of the decimal
  literals in the source, and states the contract points of PHY00a 3.3 directly.
- `BUILD` and `BUILD_windows` run it under every available compiler.
- Before the change the old port failed 9215 `sin`, 8265 `cos`, 9615 `tan` and 6 `atan2`
  rows (radians, degrees, sqrt and atan: 0).

### Also released in this version (previously Unreleased)

#### Changed

- Return `atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

#### Fixed

- Preserve negative zero and both signs of the minimum subnormal in `atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.

#### Tests

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

All notable changes to the C `trig` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `trig` crate — trigonometric functions
  computed from first principles with no `<math.h>` / libm.
- `trig_sin` / `trig_cos` (20-term Maclaurin series with range reduction into
  `[-pi, pi]`), `trig_tan` (sin/cos with pole saturation), `trig_atan` /
  `trig_atan2` (Taylor series with two-layer range reduction), `trig_radians` /
  `trig_degrees`, and `TRIG_PI`.
- `trig_sqrt` via Newton's method, using the `TrigStatus` out-parameter API
  (`TRIG_OK` / `TRIG_ERR_DOMAIN`) in place of the Rust panic on negative input.
- libm-free helpers for absolute value, truncation, and floating-point
  remainder (range reduction) so the package has zero external dependencies.
- 50 checks over sin/cos/tan/sqrt/atan/atan2/conversions (known values, parity,
  periodicity, the Pythagorean identity, and non-finite/NaN inputs), run under
  every available C compiler via the shared `iso-harness`.
- Range-reduction truncation guards NaN out of the `double`->`long long` cast
  (converting NaN to an integer is undefined behavior).
