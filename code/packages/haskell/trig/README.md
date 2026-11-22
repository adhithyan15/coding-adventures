# trig

## Exact small arctangent

`atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions implemented from first principles with basic
arithmetic.

## API

One algorithm (**TRIG-REF-1**, [PHY00a](../../../specs/PHY00a-trig-convergence.md)) is
shared by every language port, so this package returns the same bits as the others.
That is checked against the shared vectors in
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md).

- `piValue`, `twoPi`, and `halfPi` provide the shared angle constants, pinned by
  bit pattern (`Trig.Constants` holds every constant of the algorithm).
- `sin x`, `cos x`: sine and cosine in radians for `|x| <= 2^29`; NaN for NaN, the
  infinities and larger arguments. Within 3 ulp of the exact value (worst case
  found: 2.38 ulp). `sin (-0.0)` is `-0.0`; `|x| < 2^-27` returns `x`.
- `radians`, `degrees`: one multiply by the bit-pinned `pi/180` and `180/pi`.
- `tan`: `sin / cos`, with a documented `+-1e308` sentinel where `|cos| < 1e-15`
  (a pinned legacy contract, not an approximation; no accuracy claim).
- `sqrt`: Newton's method; a negative input is `Left`.
- `atan`, `atan2`: half-angle reduction and series; `atan2` is NaN when either
  argument is NaN, and keeps its pinned (non-IEEE) conventions, for example
  `atan2 (-0) (-1) = pi` and `atan2 0 0 = 0`.

## How `sin` and `cos` work

A sine repeats, so an angle is split into whole quarter-turns and a small leftover:

    x = k * (pi/2) + r,   |r| <= pi/4,   k = nearest integer to x / (pi/2)

- `k` is `(x * (2/pi) + MAGIC) - MAGIC` with `MAGIC = 1.5 * 2^52`: the add and
  subtract round to the nearest integer with only `+` and `-`.
- `r` comes from subtracting `k * piece` for five pieces of `pi/2` (the
  Cody-Waite device: the first four have 24 bits, so each product is exact).
- The kernels are the Maclaurin series on `|r| <= pi/4`, ten terms, by Horner's
  rule on stored coefficients `1/3!, 1/5!, ...`; no division anywhere.
- `k mod 4` picks `sin r`, `cos r`, `-sin r` or `-cos r` (cosine: `cos r`,
  `-sin r`, `-cos r`, `sin r`).

Every constant is a bit pattern (`castWord64ToDouble`), every operation is one IEEE
rounding, and every product that feeds a sum has its own name, so nothing can be
fused or reassociated. GHC (9.4) does neither; the test suite gives identical bits at
`-O0`, `-O1` and `-O2`.

## Dependencies

The library depends only on `base`. Tests use Hspec and read the shared vectors
from `../../../specs/fixtures/trig-reference-v1` (so run them from the package
directory, which `cabal test` does).

## Running the tests

```sh
cabal test all
```
