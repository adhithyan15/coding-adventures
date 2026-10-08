# Bezier2D (Java)

This pure G2D02 library builds quadratic and cubic curves from Point2D. It
needs no trig or host authority. De Casteljau interpolation evaluates and
splits curves; derivative roots give tight bounding boxes.

```java
QuadraticBezier arch = new QuadraticBezier(
        new Point(0, 0), new Point(1, 2), new Point(2, 0));
Point middle = arch.evaluate(0.5); // (1, 1)
List<Point> line = arch.toPolyline(0.1);
```

Flattening tests both controls against each finite chord segment, not just a
midpoint or infinite line. Invalid tolerance/coordinates fail before work;
depth 32 and 65,535 splits are hard limits with no returned partial line.
Tests consume all three polynomial and nine flattening neutral cases. Run
`gradle --no-daemon --no-build-cache --max-workers=1 check`; JaCoCo gates 95%
line coverage. Gradle writes `gradle-build/` to avoid `BUILD` collisions.
