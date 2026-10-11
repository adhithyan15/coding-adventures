# Point2D (Kotlin)

This Kotlin G2D00 package supplies immutable `Point` and `Rect` data values
for the later Affine2D, Bezier2D, and Arc2D layers. A point is both a position
and a displacement; each operation returns a fresh value. The only local
dependency is PHY00 `trig` for `angle()`.

```kotlin
import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect

val position = Point(3.0, 4.0)
val direction = position.normalize() // Point(0.6, 0.8)
val angle = direction.angle()         // local Trig.atan2(y, x)
val viewport = Rect(0.0, 0.0, 10.0, 10.0)
val inside = viewport.containsPoint(position) // right/bottom excluded
```

Lengths below `1e-12` normalize to exact origin; at the boundary they
normalize normally. Empty rectangles are union identities, and a boundary
touch yields `null` intersection. No painting or host capability is included.

Run `gradle test jacocoTestReport jacocoTestCoverageVerification`. Native
tests cover the G2D00 laws and directly consume the four checked Point2D
normalization fixtures in the mixed `geometry2d-v1` corpus. The line coverage
gate is 95%. Gradle output goes to `gradle-build/`, avoiding a `build/` versus
`BUILD` collision on case-insensitive filesystems.
