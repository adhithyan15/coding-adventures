# Point2D (Dart)

The Dart G2D00 package defines immutable Cartesian `Point` and half-open
axis-aligned `Rect` values. It is the pure geometry foundation for later
affine, Bezier, and arc packages. Angles delegate to the same-language PHY00
`coding_adventures_trig` dependency; other operations use only arithmetic.

```dart
import 'package:coding_adventures_point2d/point2d.dart';

final position = Point(3, 4);
final direction = position.normalize(); // Point(0.6, 0.8)
final angle = direction.angle();         // PHY00 atan2(y, x)
final viewport = Rect(0, 0, 10, 10);
final inside = viewport.containsPoint(position); // half-open bounds
```

Vectors shorter than `1e-12` normalize to the exact origin; a vector exactly
at the threshold still normalizes. An empty rectangle is a union identity;
intersection returns `null` for zero-area contact. This package requires no
runtime filesystem, network, process, or console authority.

Run `dart pub get`, `dart format --output=none --set-exit-if-changed lib test`,
`dart analyze --fatal-infos`, and
`dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95`.
The native suite covers G2D00 and directly reads all four checked
`point-normalize` cases from the mixed `geometry2d-v1` fixture corpus.
