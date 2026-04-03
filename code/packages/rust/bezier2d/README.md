# bezier2d

Quadratic and cubic Bezier curves — pure polynomial arithmetic on 2D points.

## Overview

This crate provides:
- **`QuadraticBezier`** (3 control points): evaluate, derivative, split, to_polyline, bounding_box, elevate
- **`CubicBezier`** (4 control points): evaluate, derivative, split, to_polyline, bounding_box

All evaluation uses de Casteljau's algorithm for numerical stability.

## Layer

G2D02 — depends on `point2d` (G2D00). No trig dependency for curve math.
