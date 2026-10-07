# point2d (Elixir)

Immutable 2D Point/Vector and Rect (AABB). G2D00.

`Point2D.normalize/1` returns the origin for magnitudes below `1.0e-12`.
At exactly `1.0e-12`, it preserves direction and returns a unit vector:

```elixir
Point2D.normalize({5.0e-13, 0.0}) # => {0.0, 0.0}
Point2D.normalize({1.0e-12, 0.0}) # => approximately {1.0, 0.0}
```
