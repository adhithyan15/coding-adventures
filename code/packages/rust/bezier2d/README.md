# bezier2d

Quadratic and cubic Bezier curves — pure polynomial arithmetic on 2D points.

## Overview

This crate provides:
- **`QuadraticBezier`** (3 control points): evaluate, derivative, split, to_polyline, bounding_box, elevate
- **`CubicBezier`** (4 control points): evaluate, derivative, split, to_polyline, bounding_box

All evaluation uses de Casteljau's algorithm for numerical stability.

## Safe flattening

`to_polyline` returns `Result<Vec<Point>, FlatteningError>`. It accepts only
finite controls and a finite positive tolerance, measures every control point
against the finite endpoint segment, and preserves parameter order without
duplicate joins. It fails atomically when derived geometry becomes non-finite
or work would exceed depth 32 or 65,535 subdivisions.

```rust
let curve = QuadraticBezier::new(p0, control, p2);
let points = curve.to_polyline(0.1)?;
```

## Layer

G2D02 — depends on `point2d` (G2D00). No trig dependency for curve math.
