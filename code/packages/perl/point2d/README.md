# point2d (Perl)

Immutable 2D Point/Vector and Rect (AABB). G2D00.

`Point->normalize` returns a fresh origin for magnitudes below `1e-12`.
At exactly `1e-12`, it preserves direction and returns a unit vector:

```perl
new_point(5e-13, 0)->normalize;  # (0.0, 0.0)
new_point(1e-12, 0)->normalize;  # approximately (1.0, 0.0)
```
