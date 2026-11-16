# Changelog

All notable changes to the C++ `trig` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.2.0] - 2026-11-16

Converges the package on the shared reference algorithm TRIG-REF-1
([PHY00a](../../../specs/PHY00a-trig-convergence.md), slice TC-5): every `sin`,
`cos`, `tan`, `radians`, `degrees`, `sqrt`, `atan` and `atan2` result now matches
the 62,154 vectors of
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md) **bit for
bit**, as in the Python, Rust and TypeScript ports. Before this change 27,101 of
those rows differed (`sin` 9,215 of 12,408; `cos` 8,265; `tan` 9,615; `atan2` 6 of 54;
`radians`, `degrees`, `sqrt` and `atan` 0).

### Changed (numeric behaviour: read this before upgrading)

- **`sin` and `cos` are a new algorithm; the last bits change for every
  argument, and large arguments change materially.** Nearest-integer Cody-Waite
  range reduction (five pieces of pi/2, four of at most 24 bits, exact products up to
  2^29), then ten-term Horner kernels on stored coefficients, replacing the 20-term
  recurrence over an `x - 2*pi*trunc(x/2*pi)` reduction. No division occurs in
  `sin`, `cos`, `radians` or `degrees`. Accuracy is within 3 ulp of the exact value
  (2.38 observed) over the whole domain; the old code was off by about 3e-12 at 1e5
  and 5e-11 at 1e6 and returned `sin(1e20) == 0`.
- **`sin`, `cos` and `tan` return NaN for `|x| > 2^29` (about 5.4e8 radians)**, for
  NaN and for both infinities. They used to return a finite, meaningless number
  beyond that. A caller that accumulates a phase must reduce it first.
- **`sin(-0.0)` is now `-0.0`** (it was `+0.0`); `|x| < 2^-27` returns `x` exactly
  (and `cos` returns `1`), including every subnormal.
- **`tan` follows the new `sin`/`cos`**, keeping its documented pole sentinel
  (`+-1e308` when `|cos| < 1e-15`; a pinned legacy contract, not an approximation,
  and no accuracy is claimed for `tan`). `tan` of anything outside the domain is NaN.
- **`atan2` returns NaN when either argument is NaN** (it used to fall through to
  `0.0`). Its other pinned conventions are unchanged and now checked by vectors:
  `atan2(-0, x<0) = +PI`, `atan2(+-0, +-0) = +0`.
- `radians` and `degrees` keep their one-multiply form but now multiply by constants
  pinned by bit pattern (`RAD_PER_DEG`, `DEG_PER_RAD`); results are unchanged.
- The header now turns off fused multiply-add for its own code, per compiler
  (`#pragma GCC optimize` on GCC, `_Pragma("STDC FP_CONTRACT OFF")` in each function
  on clang, `#pragma fp_contract(off)` on MSVC), and refuses `-ffast-math`,
  `-ffinite-math-only` and `FLT_EVAL_METHOD != 0` with `#error` / `static_assert`.
  The old truncating helpers (`d_trunc`, `d_fmod`, `range_reduce`) are removed.
- The constants of PHY00a 3.2 (`detail::INV_PIO2`, `PIO2_1` to `PIO2_5`, `MAGIC`,
  `LIMIT`, `TINY`, `RAD_PER_DEG`, `DEG_PER_RAD`, `S[]`, `C[]`) are exposed in
  `ca::trig::detail` as hex-float literals.
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

### Added

- `tests/conformance_test.cpp`: reads the four `trig-reference-v1` TSV groups and
  checks every row's bits (NaN matches any NaN; `error` means `std::domain_error`),
  checks `sin`/`cos` within 3 ulp and `radians`/`degrees` within 1 ulp of the
  correctly rounded `cr` column (`tan` named as having no accuracy claim; any other
  op with a `cr` and no limit fails), asserts every embedded constant by its bits,
  and asserts the contract directly.
- A fusion canary in `tools/run.sh` (x86-64 with FMA, or arm64): the header built
  with `-O2 -mfma` and the compiler's default contraction must pass the vectors, and
  the same code built with the guards compiled out (`-DCA_TRIG_NO_FP_GUARDS`) and
  `-ffp-contract=fast` must fail them.

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `trig` crate (namespace
  `ca::trig`) — trigonometric functions from first principles, no `<cmath>`.
- `sin` / `cos` (20-term Maclaurin series with range reduction into `[-pi, pi]`),
  `tan` (sin/cos with pole saturation), `atan` / `atan2` (Taylor series with
  two-layer range reduction), `radians` / `degrees`, and `PI` / `TWO_PI` /
  `HALF_PI` constants.
- `sqrt` via Newton's method, throwing `std::domain_error` on negative input in
  place of the Rust panic.
- libm-free `detail` helpers for absolute value, truncation, and floating-point
  remainder so the header has zero external dependencies.
- 45 checks over sin/cos/tan/sqrt/atan/atan2/conversions (including non-finite
  / NaN inputs), run under every available C++ compiler via the shared
  `iso-harness`.
- Range-reduction truncation guards NaN out of the `double`->`long long` cast
  (converting NaN to an integer is undefined behavior).
