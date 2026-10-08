# Bezier2D (Dart)

This pure G2D02 library builds quadratic and cubic curves from Point2D. It
needs no trig or host authority. De Casteljau interpolation evaluates and
splits curves; derivative roots give tight bounding boxes.

```dart
final arch = QuadraticBezier(
    const Point(0, 0), const Point(1, 2), const Point(2, 0));
final middle = arch.evaluate(0.5); // Point(1, 1)
final line = arch.toPolyline(0.1);
```

Flattening bounds whole curves by control-point distance to each finite
chord, not a midpoint or infinite line. Invalid tolerance/coordinates fail
before work; depth 32 and 65,535 splits are hard limits with no partial line.
Tests consume all three polynomial and nine flattening neutral cases. Run
`dart pub get`, `dart analyze --fatal-infos`, and the 95%-coverage command
in `BUILD`.
