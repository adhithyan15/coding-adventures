# trig

## Exact small arctangent

`Trig.atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

F# implementation of the `trig` foundation package.

This package computes trigonometric functions from arithmetic building blocks
instead of delegating to opaque runtime helpers. It is meant to be a leaf
package for future geometry and physics work in the F# package tree.

## Algorithm (TRIG-REF-1)

This port computes the same bits as every other `trig` port: it follows the
reference algorithm of [PHY00a](../../../specs/PHY00a-trig-convergence.md).

- `sin` and `cos` reduce the angle to `[-pi/4, pi/4]` by nearest-integer
  Cody-Waite reduction (`pi/2` stored as five pieces, so `k * piece` is exact),
  then sum a ten-term Horner series on stored Maclaurin coefficients, and let
  `k mod 4` pick `sin`, `cos`, `-sin` or `-cos`. Only IEEE `+`, `-`, `*` and
  comparisons are used: no fused multiply-add, no `Math.Sin`, no `%`.
- **Domain:** `sin`, `cos` and `tan` are defined for `|x| <= 2^29` (about 5.4e8
  radians). NaN, the infinities and anything larger return NaN. Reduce a
  long-running phase before calling.
- **Accuracy:** within 3 ulp of the exact value on the domain (2.38 ulp worst case
  measured); `radians` and `degrees` within 1 ulp. `tan` is `sin / cos` with a
  pinned pole sentinel (`+-1e308` where `|cos| < 1e-15`) and no accuracy claim.
- `sin(-0.0)` is `-0.0`; `0 < |x| < 2^-27` returns `x`.
- `atan2` of a NaN argument is NaN.
- The constants are pinned by bit pattern in `TrigConstants` and checked against
  the shared fixture `code/specs/fixtures/trig-reference-v1`; the test suite
  compares every vector bit for bit.

## API

- `Trig.PI`
- `Trig.sin x`
- `Trig.cos x`
- `Trig.tan x`
- `Trig.sqrt x`
- `Trig.atan x`
- `Trig.atan2 y x`
- `Trig.radians deg`
- `Trig.degrees rad`

## Usage

```fsharp
open CodingAdventures.Trig

let theta = Trig.radians 45.0
let sine = Trig.sin theta
let cosine = Trig.cos theta
let tangent = Trig.tan theta
```

## Development

```bash
bash BUILD
```
