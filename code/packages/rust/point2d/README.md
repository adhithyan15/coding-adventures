# point2d

2D point/vector and axis-aligned bounding box — the foundational geometric primitive for the G2D rendering stack.

## Overview

This crate provides two types:

- **`Point`** — a 2D position *and* a 2D vector. Operations: add, subtract, scale, negate, dot, cross, magnitude, normalize, distance, lerp, perpendicular, angle.
- **`Rect`** — an axis-aligned bounding box. Operations: union, intersection, contains_point, expand_by.

## Usage

```rust
use point2d::{Point, Rect};

let a = Point::new(3.0, 4.0);
let b = Point::new(0.0, 0.0);
println!("{}", a.distance(b));    // 5.0
println!("{:?}", a.normalize());  // Point { x: 0.6, y: 0.8 }

let r = Rect::new(0.0, 0.0, 10.0, 10.0);
println!("{}", r.contains_point(Point::new(5.0, 5.0))); // true
```

## Layer

G2D00 — depends on `trig` (PHY00).
