# trig

## Exact small arctangent

`Trig.Atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

## Algorithm: TRIG-REF-1 (bit-exact across languages)

`Sin`, `Cos`, `Radians` and `Degrees` follow the PHY00a reference algorithm
([`PHY00a-trig-convergence.md`](../../../specs/PHY00a-trig-convergence.md)), so
this port returns the same bits as every other converged `trig` port:

1. **Reduce.** `k = round(x * 2/pi)` using the add-and-subtract `MAGIC` trick,
   then `r = x - k*pi/2` by Cody-Waite subtraction of five pieces of `pi/2`
   (the first four have 24 bits, so `k * piece` is exact), leaving `|r|` near
   `pi/4`.
2. **Evaluate.** Ten-term Horner kernels on stored Maclaurin coefficients
   (`1/3!`, `1/5!`, ...), no division.
3. **Pick the quadrant** with `k & 3`.

Every operation is one IEEE binary64 operation. RyuJIT does not fuse
`a * b + c` unless `Math.FusedMultiplyAdd` is called, and this package never
calls it, nor `Math.Sin`/`Math.Cos`. Constants are built with
`BitConverter.Int64BitsToDouble`.

| Contract | Behaviour |
| --- | --- |
| Domain | `Sin`, `Cos`, `Tan` are defined for `|x| <= 2^29` (about 5.4e8); NaN, the infinities and larger arguments return NaN |
| Signed zero | `Sin(-0.0)` is `-0.0`; `|x| < 2^-27` returns `x` (`Sin`) or `1` (`Cos`) |
| Accuracy | `Sin`, `Cos` within 3 ulp of the correctly rounded value (observed worst 2); `Radians`, `Degrees` within 1 ulp; `Tan` makes no accuracy claim and keeps its `+-1e308` pole sentinel |
| `Atan2` | NaN if either argument is NaN; `Atan2(-0, -1) = +PI`, `Atan2(0, 0) = 0` |

Conformance is checked against the 62,154 shared vectors in
`code/specs/fixtures/trig-reference-v1` (bit for bit) by
`tests/CodingAdventures.Trig.Tests/ConformanceTests.cs`.

C# implementation of the `trig` foundation package.

This package computes `sin`, `cos`, `tan`, `sqrt`, `atan`, and `atan2`
without delegating to the host runtime's trigonometry helpers. It is intended
as a leaf-level teaching package that other geometry and physics packages can
build on.

## API

- `Trig.PI`
- `Trig.Sin(x)`
- `Trig.Cos(x)`
- `Trig.Tan(x)`
- `Trig.Sqrt(x)`
- `Trig.Atan(x)`
- `Trig.Atan2(y, x)`
- `Trig.Radians(deg)`
- `Trig.Degrees(rad)`

## Usage

```csharp
using CodingAdventures.Trig;

var theta = Trig.Radians(45.0);
var sine = Trig.Sin(theta);
var cosine = Trig.Cos(theta);
var tangent = Trig.Tan(theta);
```

## Development

```bash
bash BUILD
```
