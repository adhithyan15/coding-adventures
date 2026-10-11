# Affine2D (Kotlin)

This pure G2D01 library stores an affine transform as immutable SVG-order
`[a,b,c,d,e,f]` scalars. It depends on this lane's Point2D and PHY00 trig.

```kotlin
val screen = Affine2D.translate(10.0, 5.0).multiply(Affine2D.scale(2.0, 2.0))
val pixel = screen.applyToPoint(Point(3.0, 4.0)) // Point(16, 13)
val direction = screen.applyToVector(Point(1.0, 0.0)) // Point(2, 0)
```

`multiply` applies its right operand first; vectors omit translation. Inversion
returns `null` for `abs(determinant) < 1e-12`. The native tests read exactly
the five G2D01 cases in `geometry2d-v1` and gate JaCoCo line coverage at 95%.
Run `gradle --no-daemon --no-build-cache --max-workers=1 check` here. Output
uses `gradle-build/` to avoid case-insensitive collision with `BUILD`.
