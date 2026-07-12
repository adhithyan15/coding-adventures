# coding_adventures_pixel_container

IC00's universal RGBA8 pixel buffer and image codec interface, implemented in
pure Dart with zero runtime dependencies.

`PixelContainer` stores four bytes per pixel in row-major red, green, blue,
alpha order. `ImageCodec` is the shared interface implemented by encoders and
decoders throughout the image-codec family.

## Usage

```dart
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';

final pixels = PixelContainer(4, 4);
pixels.fill(255, 255, 255, 255);
pixels.setPixel(1, 1, 255, 0, 0, 255);

final (red, green, blue, alpha) = pixels.pixelAt(1, 1);
```

The package also exports `createPixelContainer`, `pixelAt`, `setPixel`, and
`fillPixels` top-level helpers for parity with the TypeScript API.

## Development

```text
dart pub get
dart format --output=none --set-exit-if-changed .
dart analyze
dart test
```
