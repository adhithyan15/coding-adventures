# Bezier2D (Kotlin)

This pure G2D02 library builds quadratic and cubic curves from Point2D. It
needs no trig or host authority. De Casteljau interpolation evaluates and
splits curves; derivative roots give tight bounding boxes.

```kotlin
val arch = QuadraticBezier(Point(0.0, 0.0), Point(1.0, 2.0), Point(2.0, 0.0))
val middle = arch.evaluate(0.5) // Point(1, 1)
val line = arch.toPolyline(0.1)
```

Flattening bounds whole curves by control-point distance to each finite
chord, not a midpoint or infinite line. Invalid tolerance/coordinates fail
before work; depth 32 and 65,535 splits are hard limits with no partial line.
Tests consume all three polynomial and nine flattening neutral cases. Run
`gradle --no-daemon --no-build-cache --max-workers=1 check`; JaCoCo gates 95%
line coverage. Gradle writes `gradle-build/` to avoid `BUILD` collisions.
