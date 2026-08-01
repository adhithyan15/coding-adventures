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
