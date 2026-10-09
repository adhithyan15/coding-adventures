# Arc2D (Dart)

G2D03 implements SVG endpoint arcs and bounded center-form elliptical arcs using
Point2D, Bezier2D, and the local PHY00 trigonometry package. SVG degeneracies
fall back to a line or point; direct center arcs require positive finite radii
and no more than one turn.

```dart
final arc = SvgArc(const Point(1, 0), const Point(0, 1), 1, 1, 0, false, true);
final midpoint = arc.evaluate(0.5);
final curves = arc.toCubicBeziers();
```

`boundingBox` uses analytic extrema and directed periodic membership, not
sampling. Cubic conversion emits one to four finite segments or fails before
allocation. Tests consume all 11 Arc2D records in `geometry2d-v1` plus native
W3C, threshold, rotation, and non-finite cases. Run the `BUILD` commands for
format, analysis, and the 95% coverage gate.
