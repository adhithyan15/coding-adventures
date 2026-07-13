# coding_adventures_image_geometric_transforms

IMG04 spatial transforms for the shared RGBA8 `PixelContainer`, implemented
in pure Dart. It provides exact byte-preserving operations and continuous
inverse warps with configurable sampling.

## Features

- Lossless `flipHorizontal`, `flipVertical`, `rotate90CW`, `rotate90CCW`,
  `rotate180`, `crop`, and `pad`
- Continuous `scale`, `rotate`, `affine`, and `perspectiveWarp`
- Nearest, bilinear, and Catmull-Rom bicubic interpolation
- Zero, replicate, reflect, and wrap out-of-bounds policies
- Linear-light RGB interpolation with linear alpha blending
- Pixel-centre alignment for scaling and inverse-warp coverage without holes

## Usage

```dart
import 'package:coding_adventures_image_geometric_transforms/coding_adventures_image_geometric_transforms.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';

final source = PixelContainer(2, 1)
  ..setPixel(0, 0, 255, 0, 0, 255)
  ..setPixel(1, 0, 0, 0, 255, 255);

final mirrored = flipHorizontal(source);
final enlarged = scale(source, 8, 4);
final turned = rotate(
  source,
  0.5,
  interpolation: Interpolation.bicubic,
  bounds: RotateBounds.fit,
);
```

Affine matrices are 2×3 inverse-mapping matrices. Perspective matrices are
3×3 inverse homographies. Matrix literals may contain either integer or
floating-point values.

## Development

```text
dart pub get
dart format --output=none --set-exit-if-changed .
dart analyze
dart test
```
