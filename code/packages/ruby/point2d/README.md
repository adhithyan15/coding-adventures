# point2d (Ruby)

Immutable 2D Point/Vector and Rect (AABB). G2D00.

`Point#normalize` returns a fresh origin for magnitudes below `1e-12`.
At exactly `1e-12`, it preserves direction and returns a unit vector:

```ruby
Point2D::Point.new(5e-13, 0).normalize # => Point2D::Point.new(0.0, 0.0)
Point2D::Point.new(1e-12, 0).normalize # => approximately (1.0, 0.0)
```
