# Affine2D (Dart)

This pure G2D01 library stores an affine transform as immutable SVG-order
`[a,b,c,d,e,f]` scalars. It depends on this lane's Point2D and PHY00 trig.

```dart
final screen = Affine2D.translate(10, 5).multiply(Affine2D.scale(2, 2));
final pixel = screen.applyToPoint(const Point(3, 4)); // Point(16, 13)
final direction = screen.applyToVector(const Point(1, 0)); // Point(2, 0)
```

`multiply` applies its right operand first; vectors omit translation. Inversion
returns `null` for `abs(determinant) < 1e-12`. Tests read all five G2D01
neutral cases. Run `dart pub get`, `dart analyze --fatal-infos`, and the
coverage-gated command in `BUILD` (95% floor).
