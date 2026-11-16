# Changelog

## [0.4.0] - 2026-11-16

### Changed (numeric behaviour: TRIG-CONV TC-6, PHY00a)

- `Sin` and `Cos` now implement the TRIG-REF-1 reference algorithm and return the
  same bits as every other converged port: Cody-Waite range reduction to
  `|r| <= pi/4` (nearest quarter turn through the `1.5 * 2^52` add-and-subtract,
  five-piece `pi/2`), then a ten-term Horner polynomial. The 21-term series on
  `x - 2pi*int(x/2pi)` is gone. Last-bit differences everywhere; worst error
  against the correctly rounded value is 2.38 ulp for `sin` and 2.37 ulp for `cos`
  (gate: 3 ulp), against a loss of precision that grew with `|x|` before.
- **`Sin`, `Cos` and `Tan` return NaN for `|x| > 2^29`** (and, as before, for NaN
  and infinities), where `Sin(1e20)` used to return an implementation-defined
  number (NaN on amd64, a saturated-conversion result on arm64). A caller with an
  unbounded phase must reduce it first.
- `Radians` and `Degrees` are a single multiplication by the stored constants
  `pi/180` and `180/pi` (0.65 and 0.81 ulp worst case), not `deg*PI/180`; this no
  longer overflows for `deg > 5.7e307`.
- `Atan2` returns NaN when either argument is NaN (it returned `0.0`).
- `Sin(-0)` is `-0` and `Sin(x) == x` for `|x| < 2^-27`; `Cos` of that is exactly 1.
- Every product that feeds an addition or subtraction is written
  `float64(x*y)`. Go may otherwise fuse it into an FMA on arm64, ppc64, s390x and
  riscv64; the BUILD file now checks that the arm64 and `GOAMD64=v3` assembly
  contains no `FMADD`/`FMSUB`/`FNMSUB`/`VFMADD`.
- `Sqrt` and `atanCore` keep their algorithm; only their fusable products
  (`1.0 + x*x`, `1e-15*guess + 1e-300`, the halving in the Newton step) gain the
  explicit conversion, so their bits do not depend on the architecture.
- The internal `rangeReduce` helper is replaced by `reduce`.

### Tests

- `conformance_test.go` checks every row of the four shared groups in
  `code/specs/fixtures/trig-reference-v1` bit for bit (NaN as a class), the
  manifest hashes, the constants' bits, accuracy against the correctly rounded
  column (`sin`, `cos` 3 ulp, `radians`, `degrees` 1 ulp; `tan` is the named
  no-claim op), and the contract points of PHY00a 3.3.

### Carried over from the earlier Unreleased section

Changed:

- Return `Atan(x)` unchanged for `|x| <= 2^-27` before half-angle reduction,
  preserving the exact binary64 small-argument identity.
- Normalize square-root inputs by powers of four before Newton iteration so the
  complete binary64 exponent range converges without host square-root calls.

Fixed:

- Preserve negative zero and both signs of the minimum subnormal in `Atan`.
- Preserve negative zero, return positive infinity, propagate NaN, and retain
  the lane-native negative-input error.
- Move negative-input rejection outside the operation callback so `Sqrt`
  propagates its documented panic instead of returning the operation fallback.

Tests:

- Cover the shared PHY00 `atan` signed-zero and tiny/subnormal boundaries.
- Cover the shared PHY00 square-root boundaries from
  [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json) with
  relative-error assertions for finite nonzero results.

## [0.3.0] - 2026-04-03

### Added

- `Sqrt(x float64)` — square root via Newton's method; panics for negative inputs.
- `Tan(x float64)` — tangent as Sin/Cos ratio with pole guard.
- `Atan(x float64)` — arctangent via Taylor series with outer and half-angle range reduction.
- `Atan2(y, x float64)` — four-quadrant arctangent.
- `halfPI` private constant.
- `atanCore(x float64)` private helper for inner atan computation.
- Tests for all new functions in `trig_test.go`.

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All four public functions (`Sin`, `Cos`,
  `Radians`, `Degrees`) are now wrapped with `StartNew[float64]` from the
  package's Operations infrastructure. Each call gains automatic timing,
  structured logging, and panic recovery.

## [0.1.0] - 2026-03-22

### Added
- `PI` constant to double-precision accuracy
- `Sin(x)` via Maclaurin series with range reduction
- `Cos(x)` via Maclaurin series with range reduction
- `Radians(deg)` degree-to-radian conversion
- `Degrees(rad)` radian-to-degree conversion
