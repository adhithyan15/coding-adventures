# coding_adventures_image_point_ops

IMG03 per-pixel point operations for the shared `PixelContainer` image type,
implemented in pure Dart. Each operation returns a new RGBA8 image and preserves
alpha unless alpha is the selected channel.

The package includes encoded-byte operations (`invert`, thresholds, posterize,
channel operations, brightness), linear-light transforms (contrast, gamma,
exposure, greyscale, sepia, colour matrices, saturation, hue rotation), sRGB
conversion helpers, and one-dimensional LUT builders/applicators.

## Usage

```dart
import 'package:coding_adventures_image_point_ops/coding_adventures_image_point_ops.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';

final image = PixelContainer(1, 1)..setPixel(0, 0, 10, 100, 200, 255);
final inverted = invert(image);
final monochrome = greyscale(image);
```

## Development

```text
dart pub get
dart format --output=none --set-exit-if-changed .
dart analyze
dart test
```
