# point2d (Swift)

Immutable 2D Point/Vector and Rect (AABB). G2D00.

`Point.normalize()` returns the origin for magnitudes below `1e-12`.
At exactly `1e-12`, it preserves direction and returns a unit vector:

```swift
Point(5e-13, 0).normalize() // Point(0.0, 0.0)
Point(1e-12, 0).normalize() // approximately Point(1.0, 0.0)
```
