import 'dart:math' as math;

import 'package:coding_adventures_image_geometric_transforms/coding_adventures_image_geometric_transforms.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';
import 'package:test/test.dart';

PixelContainer gradient(int width, int height) {
  final image = PixelContainer(width, height);
  for (var y = 0; y < height; y++) {
    for (var x = 0; x < width; x++) {
      image.setPixel(x, y, x * 10, y * 10, (x + y) * 5, 255);
    }
  }
  return image;
}

PixelContainer solid(int width, int height, Rgba8 pixel) =>
    PixelContainer(width, height)..fill(pixel.$1, pixel.$2, pixel.$3, pixel.$4);

void expectPixelNear(Rgba8 actual, Rgba8 expected, {int tolerance = 2}) {
  for (final difference in [
    (actual.$1 - expected.$1).abs(),
    (actual.$2 - expected.$2).abs(),
    (actual.$3 - expected.$3).abs(),
    (actual.$4 - expected.$4).abs(),
  ]) {
    expect(difference, lessThanOrEqualTo(tolerance));
  }
}

void main() {
  group('lossless transforms', () {
    test('horizontal and vertical double flips are exact identities', () {
      final source = gradient(5, 3);
      expect(flipHorizontal(flipHorizontal(source)), source);
      expect(flipVertical(flipVertical(source)), source);
    });

    test('flips reverse the expected axes', () {
      final source = gradient(4, 3);
      final horizontal = flipHorizontal(source);
      final vertical = flipVertical(source);
      for (var y = 0; y < source.height; y++) {
        for (var x = 0; x < source.width; x++) {
          expect(horizontal.pixelAt(x, y), source.pixelAt(3 - x, y));
          expect(vertical.pixelAt(x, y), source.pixelAt(x, 2 - y));
        }
      }
    });

    test('right-angle rotations swap dimensions and round-trip', () {
      final source = gradient(4, 3);
      final clockwise = rotate90CW(source);
      expect((clockwise.width, clockwise.height), (3, 4));
      expect(clockwise.pixelAt(0, 0), source.pixelAt(0, 2));
      expect(rotate90CCW(clockwise), source);
      expect(rotate180(rotate180(source)), source);
    });

    test('four clockwise rotations are an exact identity', () {
      final source = gradient(5, 3);
      var output = source;
      for (var iteration = 0; iteration < 4; iteration++) {
        output = rotate90CW(output);
      }
      expect(output, source);
    });

    test('crop extracts pixels and uses transparent black out of bounds', () {
      final source = gradient(5, 5);
      final output = crop(source, 2, 1, 3, 2);
      expect((output.width, output.height), (3, 2));
      expect(output.pixelAt(0, 0), source.pixelAt(2, 1));
      expect(crop(source, 20, 20, 1, 1).pixelAt(0, 0), transparent);
      expect(() => crop(source, 0, 0, -1, 2), throwsRangeError);
    });

    test('pad fills the border and preserves the interior', () {
      final source = gradient(4, 3);
      final output = pad(source, 1, 2, 3, 4, (255, 0, 0, 255));
      expect((output.width, output.height), (10, 7));
      expect(output.pixelAt(0, 0), (255, 0, 0, 255));
      expect(output.pixelAt(4, 1), source.pixelAt(0, 0));
      expect(() => pad(source, -1, 0, 0, 0, transparent), throwsRangeError);
    });
  });

  group('sampling', () {
    test('nearest reads exact pixels and zero-fills out of bounds', () {
      final source = PixelContainer(4, 4)..setPixel(2, 1, 111, 222, 33, 200);
      expect(sample(source, 2, 1, Interpolation.nearest, OutOfBounds.zero), (
        111,
        222,
        33,
        200,
      ));
      expect(
        sample(source, -1, 0, Interpolation.nearest, OutOfBounds.zero),
        transparent,
      );
    });

    test('bilinear blends RGB in linear light and alpha linearly', () {
      final source = PixelContainer(2, 1)
        ..setPixel(0, 0, 0, 0, 0, 255)
        ..setPixel(1, 0, 255, 255, 255, 127);
      final result = sample(
        source,
        0.5,
        0,
        Interpolation.bilinear,
        OutOfBounds.replicate,
      );
      expect(result.$1, inInclusiveRange(184, 192));
      expect(
        (result.$1, result.$2, result.$3),
        (result.$2, result.$3, result.$1),
      );
      expect(result.$4, 191);
    });

    test('bilinear and bicubic reproduce integer coordinates', () {
      final source = gradient(8, 8);
      final expected = source.pixelAt(3, 3);
      expectPixelNear(
        sample(source, 3, 3, Interpolation.bilinear, OutOfBounds.replicate),
        expected,
      );
      expectPixelNear(
        sample(source, 3, 3, Interpolation.bicubic, OutOfBounds.replicate),
        expected,
        tolerance: 3,
      );
    });

    test('all out-of-bounds policies map coordinates correctly', () {
      final source = PixelContainer(4, 1)
        ..setPixel(0, 0, 10, 20, 30, 255)
        ..setPixel(1, 0, 40, 50, 60, 255)
        ..setPixel(3, 0, 70, 80, 90, 255);
      Rgba8 read(num x, OutOfBounds policy) =>
          sample(source, x, 0, Interpolation.nearest, policy);

      expect(read(-5, OutOfBounds.replicate), (10, 20, 30, 255));
      expect(read(4, OutOfBounds.wrap), (10, 20, 30, 255));
      expect(read(-1, OutOfBounds.reflect), (10, 20, 30, 255));
      expect(read(4, OutOfBounds.reflect), (70, 80, 90, 255));
      expect(read(99, OutOfBounds.zero), transparent);
    });

    test('rejects non-finite coordinates', () {
      final source = solid(1, 1, (0, 0, 0, 0));
      expect(
        () => sample(
          source,
          double.nan,
          0,
          Interpolation.nearest,
          OutOfBounds.zero,
        ),
        throwsArgumentError,
      );
    });
  });

  group('continuous transforms', () {
    test('scale changes dimensions and preserves a one-pixel image', () {
      final source = solid(1, 1, (100, 150, 200, 255));
      final output = scale(source, 7, 5, interpolation: Interpolation.nearest);
      expect((output.width, output.height), (7, 5));
      expect(output.pixelAt(6, 4), (100, 150, 200, 255));
    });

    test('zero-angle crop rotation is identity', () {
      final source = gradient(8, 8);
      expect(
        rotate(
          source,
          0,
          interpolation: Interpolation.nearest,
          bounds: RotateBounds.crop,
        ),
        source,
      );
    });

    test('fit rotation expands while crop rotation preserves dimensions', () {
      final source = gradient(10, 10);
      final fitted = rotate(source, math.pi / 4);
      final cropped = rotate(source, math.pi / 6, bounds: RotateBounds.crop);
      expect(fitted.width, greaterThan(source.width));
      expect(fitted.height, greaterThan(source.height));
      expect((cropped.width, cropped.height), (10, 10));
    });

    test('identity affine matrix reproduces every pixel', () {
      final source = gradient(6, 6);
      final output = affine(
        source,
        const [
          [1, 0, 0],
          [0, 1, 0],
        ],
        6,
        6,
        interpolation: Interpolation.nearest,
        oob: OutOfBounds.replicate,
      );
      expect(output, source);
    });

    test('affine validates shape and finite coefficients', () {
      final source = gradient(2, 2);
      expect(
        () => affine(
          source,
          const [
            [1, 0],
          ],
          2,
          2,
        ),
        throwsArgumentError,
      );
      expect(
        () => affine(
          source,
          const [
            [1, 0, double.nan],
            [0, 1, 0],
          ],
          2,
          2,
        ),
        throwsArgumentError,
      );
    });

    test('identity homography reproduces every pixel', () {
      final source = gradient(6, 6);
      final output = perspectiveWarp(
        source,
        const [
          [1, 0, 0],
          [0, 1, 0],
          [0, 0, 1],
        ],
        6,
        6,
        interpolation: Interpolation.nearest,
        oob: OutOfBounds.replicate,
      );
      expect(output, source);
    });

    test('zero projective weight produces transparent output', () {
      final source = solid(2, 2, (255, 255, 255, 255));
      final output = perspectiveWarp(
        source,
        const [
          [1, 0, 0],
          [0, 1, 0],
          [0, 0, 0],
        ],
        2,
        2,
      );
      expect(output.data, everyElement(0));
    });

    test('reject an empty source image', () {
      expect(() => scale(PixelContainer(0, 0), 1, 1), throwsStateError);
      expect(() => rotate(PixelContainer(0, 1), 0), throwsStateError);
    });
  });
}
