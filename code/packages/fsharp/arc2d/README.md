# CodingAdventures.Arc2D.FSharp

Elliptical arcs with SVG endpoint conversion, center-form evaluation, bounds, tangents, and cubic Bezier approximation.

`CenterArc.BoundingBox()` evaluates rotated x/y extrema analytically rather
than sampling fixed parameters; it handles wrapped positive and negative
sweeps. Direct center arcs require finite center coordinates, positive finite
radii, finite angles and a sweep no longer than one turn. A finite over-turn
throws `ArgumentOutOfRangeException`; other invalid/non-finite inputs and
derived output throw `ArgumentException`. A zero sweep emits one degenerate
cubic; a full turn emits exactly four.

```fsharp
let arc = CenterArc.New(Point.Origin(), 2.0, 1.0, 0.0, Trig.PI / 2.0, 0.0)
let bounds = arc.BoundingBox()
let cubics = arc.ToCubicBeziers() // one segment
```

The test suite consumes the seven shared `geometry2d-v1` center-form cases
at runtime and separately tests native NaN/infinity rejection.

For degenerate `SvgArc` inputs, `ToCenterArc()` remains `None` and cubics are
empty, but `Evaluate(t)` returns `Some` endpoint-line interpolation and
`BoundingBox()` returns `Some` ordered endpoint rectangle. A coincident arc
has point bounds; SVG negative radii are corrected to absolute values.
