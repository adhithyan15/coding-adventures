# trig

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
