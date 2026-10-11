# CodingAdventures.Arc2D.CSharp

Elliptical arcs with SVG endpoint conversion, center-form evaluation, bounds, tangents, and cubic Bezier approximation.

`CenterArc.BoundingBox()` evaluates the rotated ellipse's stationary angles
analytically, including positive and negative sweeps that wrap across `2π`.
`ToCubicBeziers()` emits one through four segments for a valid sweep; zero
sweep yields one degenerate cubic and a full turn yields four. Direct center
arcs require positive finite radii, finite coordinates and angles, and at
most one turn. A finite over-turn sweep throws `ArgumentOutOfRangeException`;
other invalid or non-finite inputs and outputs throw `ArgumentException`.

```csharp
var arc = new CenterArc(new Point(0, 0), 2, 1, 0, Math.PI / 2, 0);
var bounds = arc.BoundingBox();
var cubics = arc.ToCubicBeziers(); // one segment
```

The test suite reads all seven shared `geometry2d-v1` center-form cases at
runtime as well as native non-finite-input checks.

Degenerate `SvgArc` values retain a nullable `ToCenterArc()` and empty cubic
output, while `Evaluate(t)` yields endpoint-line interpolation and
`BoundingBox()` yields an ordered endpoint rectangle. Coincident endpoints
produce a point rectangle; negative radii use their SVG absolute values.
