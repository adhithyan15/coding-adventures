# Point2D (Java)

This is the Java G2D00 leaf of the 2D geometry stack. Immutable `Point`
records act as positions or displacement vectors; immutable `Rect` records
represent half-open axis-aligned boxes. The only package dependency is the
same-language PHY00 `trig` package, used for quadrant-correct angles.

```java
import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;

Point position = new Point(3, 4);
Point direction = position.normalize();   // (0.6, 0.8)
double angle = direction.angle();          // local PHY00 atan2(y, x)
Rect viewport = new Rect(0, 0, 10, 10);
boolean inside = viewport.containsPoint(position); // true; right/bottom excluded
```

`normalize()` returns the exact origin for lengths strictly below `1e-12`;
exactly `1e-12` normalizes. `union()` treats empty boxes as identities and
`intersection()` returns `null` for a zero-area touch. No paint, host I/O, or
execution authority is included.

Run `gradle test jacocoTestReport jacocoTestCoverageVerification` here. Native
tests cover G2D00's required laws and directly read all four `point-normalize`
cases from the mixed language-neutral `geometry2d-v1` corpus. JaCoCo requires
at least 95% line coverage. Gradle output is redirected to `gradle-build/`
because the repository's uppercase `BUILD` file would collide with `build/`
on case-insensitive filesystems.
