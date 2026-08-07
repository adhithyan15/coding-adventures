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
