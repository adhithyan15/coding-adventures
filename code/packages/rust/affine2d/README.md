# affine2d

2D affine transformation matrix — the standard 6-float representation used by SVG, Canvas, PDF, and every major 2D graphics API.

## Usage

```rust
use affine2d::Affine2D;
use point2d::Point;

let t = Affine2D::translate(10.0, 5.0);
let r = Affine2D::rotate(std::f64::consts::PI / 4.0);
let composed = t.multiply(&r);
let p = composed.apply_to_point(Point::new(1.0, 0.0));
```

## Layer

G2D01 — depends on `point2d` (G2D00) and `trig` (PHY00).
