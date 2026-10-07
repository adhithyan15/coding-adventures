# point2d (Lua)

Immutable 2D Point/Vector and Rect (AABB). G2D00.

`point2d.normalize` returns a fresh origin for magnitudes below `1e-12`.
At exactly `1e-12`, it preserves direction and returns a unit vector:

```lua
point2d.normalize(point2d.new_point(5e-13, 0)) -- {x = 0.0, y = 0.0}
point2d.normalize(point2d.new_point(1e-12, 0)) -- approximately {x = 1.0, y = 0.0}
```
