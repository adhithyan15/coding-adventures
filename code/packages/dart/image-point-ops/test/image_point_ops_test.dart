import 'dart:typed_data';

import 'package:coding_adventures_image_point_ops/coding_adventures_image_point_ops.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';
import 'package:test/test.dart';

PixelContainer solid(int red, int green, int blue, int alpha) =>
    PixelContainer(1, 1)..setPixel(0, 0, red, green, blue, alpha);

void expectNear(Rgba actual, Rgba expected, {int tolerance = 1}) {
  expect((actual.$1 - expected.$1).abs(), lessThanOrEqualTo(tolerance));
  expect((actual.$2 - expected.$2).abs(), lessThanOrEqualTo(tolerance));
  expect((actual.$3 - expected.$3).abs(), lessThanOrEqualTo(tolerance));
  expect(actual.$4, expected.$4);
}

void main() {
  group('encoded-byte operations', () {
    test('invert preserves dimensions and alpha', () {
      final source = PixelContainer(3, 5)..setPixel(0, 0, 10, 100, 200, 128);
      final output = invert(source);
      expect((output.width, output.height), (3, 5));
      expect(output.pixelAt(0, 0), (245, 155, 55, 128));
    });

    test('double invert is exact identity', () {
      final source = solid(30, 80, 180, 255);
      expect(invert(invert(source)), source);
    });

    test('threshold splits below, at, and above the boundary', () {
      expect(threshold(solid(50, 50, 50, 17), 128).pixelAt(0, 0), (
        0,
        0,
        0,
        17,
      ));
      expect(threshold(solid(128, 128, 128, 18), 128).pixelAt(0, 0), (
        255,
        255,
        255,
        18,
      ));
      expect(threshold(solid(200, 200, 200, 19), 128).pixelAt(0, 0), (
        255,
        255,
        255,
        19,
      ));
    });

    test('luminance threshold uses Rec. 709 encoded-byte weights', () {
      expect(thresholdLuminance(solid(255, 0, 0, 255), 100).pixelAt(0, 0), (
        0,
        0,
        0,
        255,
      ));
      expect(thresholdLuminance(solid(0, 255, 0, 255), 100).pixelAt(0, 0), (
        255,
        255,
        255,
        255,
      ));
    });

    test('posterize creates equally spaced levels', () {
      expect(posterize(solid(0, 63, 128, 200), 3).pixelAt(0, 0), (
        0,
        0,
        128,
        200,
      ));
      expect(posterize(solid(191, 255, 50, 200), 3).pixelAt(0, 0), (
        128,
        255,
        0,
        200,
      ));
      expect(() => posterize(solid(0, 0, 0, 0), 1), throwsRangeError);
    });

    test('swapRgbBgr exchanges red and blue', () {
      expect(swapRgbBgr(solid(255, 20, 0, 128)).pixelAt(0, 0), (
        0,
        20,
        255,
        128,
      ));
    });

    test('extractChannel supports all four channel indices', () {
      final source = solid(100, 150, 200, 77);
      expect(extractChannel(source, 0).pixelAt(0, 0), (100, 0, 0, 77));
      expect(extractChannel(source, 1).pixelAt(0, 0), (0, 150, 0, 77));
      expect(extractChannel(source, 2).pixelAt(0, 0), (0, 0, 200, 77));
      expect(extractChannel(source, 3).pixelAt(0, 0), (100, 150, 200, 77));
      expect(() => extractChannel(source, 4), throwsRangeError);
    });

    test('brightness adds, rounds, and clamps', () {
      expect(brightness(solid(250, 10, 5, 99), 20).pixelAt(0, 0), (
        255,
        30,
        25,
        99,
      ));
      expect(brightness(solid(5, 10, 250, 99), -20).pixelAt(0, 0), (
        0,
        0,
        230,
        99,
      ));
    });
  });

  group('linear-light operations', () {
    final identitySource = solid(100, 150, 200, 231);

    test('contrast factor one is identity within rounding', () {
      expectNear(
        contrast(identitySource, 1).pixelAt(0, 0),
        identitySource.pixelAt(0, 0),
      );
    });

    test('gamma one is identity and gamma below one brightens', () {
      expectNear(
        gamma(identitySource, 1).pixelAt(0, 0),
        identitySource.pixelAt(0, 0),
      );
      expect(
        gamma(solid(128, 128, 128, 255), 0.5).pixelAt(0, 0).$1,
        greaterThan(128),
      );
    });

    test('positive exposure brightens without changing alpha', () {
      final output = exposure(solid(100, 100, 100, 42), 1).pixelAt(0, 0);
      expect(output.$1, greaterThan(100));
      expect(output.$4, 42);
    });

    test('all greyscale methods preserve black and white', () {
      for (final method in GreyscaleMethod.values) {
        expect(greyscale(solid(255, 255, 255, 255), method).pixelAt(0, 0), (
          255,
          255,
          255,
          255,
        ));
        expect(greyscale(solid(0, 0, 0, 128), method).pixelAt(0, 0), (
          0,
          0,
          0,
          128,
        ));
      }
    });

    test('greyscale returns equal RGB channels', () {
      final output = greyscale(solid(200, 100, 50, 255)).pixelAt(0, 0);
      expect(output.$1, output.$2);
      expect(output.$2, output.$3);
    });

    test('sepia preserves alpha and creates warm ordering', () {
      final output = sepia(solid(128, 128, 128, 200)).pixelAt(0, 0);
      expect(output.$1, greaterThanOrEqualTo(output.$2));
      expect(output.$2, greaterThan(output.$3));
      expect(output.$4, 200);
    });

    test('identity colour matrix is identity', () {
      final output = colourMatrix(identitySource, const [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
      ]);
      expectNear(output.pixelAt(0, 0), identitySource.pixelAt(0, 0));
    });

    test('colour matrix rejects invalid shape', () {
      expect(
        () => colourMatrix(identitySource, const [
          [1, 0],
          [0, 1],
        ]),
        throwsArgumentError,
      );
    });

    test('zero saturation produces equal channels', () {
      final output = saturate(solid(200, 100, 50, 255), 0).pixelAt(0, 0);
      expect(output.$1, output.$2);
      expect(output.$2, output.$3);
    });

    test('360 degree hue rotation is identity within rounding', () {
      final source = solid(200, 80, 40, 240);
      expectNear(
        hueRotate(source, 360).pixelAt(0, 0),
        source.pixelAt(0, 0),
        tolerance: 2,
      );
    });

    test('negative hue rotation wraps around the colour wheel', () {
      final source = solid(200, 80, 40, 240);
      expectNear(
        hueRotate(source, -360).pixelAt(0, 0),
        source.pixelAt(0, 0),
        tolerance: 2,
      );
    });
  });

  group('colorspace and LUT operations', () {
    test('sRGB to linear byte round trip is approximate identity', () {
      final source = solid(100, 150, 200, 255);
      final output = linearToSrgbImage(srgbToLinearImage(source));
      expectNear(output.pixelAt(0, 0), source.pixelAt(0, 0), tolerance: 2);
    });

    test('applyLut1dU8 applies independent channel tables', () {
      final invertLut = Uint8List.fromList([
        for (var index = 0; index < 256; index++) 255 - index,
      ]);
      final identityLut = Uint8List.fromList([
        for (var index = 0; index < 256; index++) index,
      ]);
      final zeroLut = Uint8List(256);
      final output = applyLut1dU8(
        solid(100, 25, 200, 123),
        invertLut,
        identityLut,
        zeroLut,
      );
      expect(output.pixelAt(0, 0), (155, 25, 0, 123));
    });

    test('applyLut1dU8 requires 256 entries per channel', () {
      final valid = Uint8List(256);
      expect(
        () => applyLut1dU8(solid(0, 0, 0, 0), Uint8List(255), valid, valid),
        throwsArgumentError,
      );
    });

    test('identity mapping builds identity LUT within rounding', () {
      final lut = buildLut1dU8((value) => value);
      expect(lut.length, 256);
      for (var index = 0; index < 256; index++) {
        expect(
          (lut[index] - index).abs(),
          lessThanOrEqualTo(1),
          reason: 'LUT entry $index',
        );
      }
    });

    test('gamma one builds identity LUT within rounding', () {
      final lut = buildGammaLut(1);
      for (var index = 0; index < 256; index++) {
        expect(
          (lut[index] - index).abs(),
          lessThanOrEqualTo(1),
          reason: 'gamma LUT entry $index',
        );
      }
    });
  });

  test('operations return independent images and do not mutate source', () {
    final source = solid(10, 20, 30, 40);
    final output = invert(source);
    output.setPixel(0, 0, 1, 2, 3, 4);
    expect(source.pixelAt(0, 0), (10, 20, 30, 40));
  });

  test('zero-sized images are accepted by every operation family', () {
    final empty = PixelContainer(0, 0);
    expect(invert(empty).data, isEmpty);
    expect(greyscale(empty).data, isEmpty);
    expect(
      applyLut1dU8(empty, Uint8List(256), Uint8List(256), Uint8List(256)).data,
      isEmpty,
    );
  });
}
