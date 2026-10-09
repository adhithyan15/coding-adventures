# Arc2D (Kotlin)

G2D03 implements SVG endpoint arcs and bounded center-form elliptical arcs using
Point2D, Bezier2D, and the local PHY00 trigonometry package. SVG degeneracies
fall back to a line or point; direct center arcs require positive finite radii
and no more than one turn.

```kotlin
val arc = SvgArc(Point(1.0, 0.0), Point(0.0, 1.0), 1.0, 1.0, 0.0, false, true)
val midpoint = arc.evaluate(0.5)
val curves = arc.toCubicBeziers()
```

`boundingBox` uses analytic extrema and directed periodic membership, not
sampling. Cubic conversion emits one to four finite segments or fails before
allocation. Tests consume all 11 Arc2D records in `geometry2d-v1` plus native
W3C, threshold, rotation, and non-finite cases. Run
`gradle --no-daemon --no-build-cache --max-workers=1 check`; JaCoCo gates 95%
line coverage. Build products go in `gradle-build/` to avoid `BUILD` collisions.
