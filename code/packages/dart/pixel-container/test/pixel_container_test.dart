import 'dart:typed_data';

import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';
import 'package:test/test.dart';

final class StubCodec implements ImageCodec {
  @override
  String get mimeType => 'image/x-stub';

  @override
  Uint8List encode(PixelContainer pixels) =>
      Uint8List.fromList([pixels.width, pixels.height, ...pixels.data]);

  @override
  PixelContainer decode(Uint8List bytes) {
    if (bytes.length < 2) throw const FormatException('stub image too short');
    return PixelContainer.fromData(
      bytes[0],
      bytes[1],
      Uint8List.fromList(bytes.sublist(2)),
    );
  }
}

void main() {
  group('construction', () {
    test('allocates width * height * 4 transparent bytes', () {
      final pixels = PixelContainer(4, 3);
      expect(pixels.width, 4);
      expect(pixels.height, 3);
      expect(pixels.pixelCount, 12);
      expect(pixels.byteCount, 48);
      expect(pixels.data, everyElement(0));
    });

    test('supports zero dimensions', () {
      expect(PixelContainer(0, 0).data, isEmpty);
      expect(PixelContainer(0, 5).data, isEmpty);
      expect(PixelContainer(5, 0).data, isEmpty);
    });

    test('rejects negative dimensions', () {
      expect(() => PixelContainer(-1, 1), throwsRangeError);
      expect(() => PixelContainer(1, -1), throwsRangeError);
    });

    test('fromData copies valid bytes', () {
      final source = Uint8List.fromList([255, 128, 64, 32]);
      final pixels = PixelContainer.fromData(1, 1, source);
      source[0] = 0;
      expect(pixels.data, [255, 128, 64, 32]);
    });

    test('fromData rejects the wrong byte count', () {
      expect(
        () => PixelContainer.fromData(1, 1, Uint8List(3)),
        throwsArgumentError,
      );
    });
  });

  group('pixel access', () {
    test('setPixel and pixelAt round-trip RGBA values', () {
      final pixels = PixelContainer(4, 4);
      pixels.setPixel(1, 2, 200, 100, 50, 255);
      expect(pixels.pixelAt(1, 2), (200, 100, 50, 255));
    });

    test('uses row-major RGBA byte layout', () {
      final pixels = PixelContainer(3, 2);
      pixels.setPixel(2, 1, 11, 22, 33, 44);
      expect(pixels.data.sublist(20, 24), [11, 22, 33, 44]);
    });

    test('out-of-bounds reads return transparent', () {
      final pixels = PixelContainer(3, 3);
      for (final coordinate in [(-1, 0), (0, -1), (3, 0), (0, 3)]) {
        expect(pixels.pixelAt(coordinate.$1, coordinate.$2), transparent);
      }
    });

    test('out-of-bounds writes are no-ops', () {
      final pixels = PixelContainer(2, 2);
      pixels.setPixel(99, 0, 1, 2, 3, 4);
      pixels.setPixel(0, -1, 1, 2, 3, 4);
      expect(pixels.data, everyElement(0));
    });

    test('valid writes reject channels outside RGBA8', () {
      final pixels = PixelContainer(1, 1);
      expect(() => pixels.setPixel(0, 0, -1, 0, 0, 0), throwsRangeError);
      expect(() => pixels.setPixel(0, 0, 0, 256, 0, 0), throwsRangeError);
    });

    test('does not modify neighbouring pixels', () {
      final pixels = PixelContainer(4, 4);
      pixels.setPixel(2, 1, 255, 0, 0, 255);
      expect(pixels.pixelAt(1, 1), transparent);
      expect(pixels.pixelAt(3, 1), transparent);
    });
  });

  group('fill', () {
    test('sets every pixel and overwrites previous data', () {
      final pixels = PixelContainer(3, 3)..setPixel(0, 0, 1, 2, 3, 4);
      pixels.fill(100, 150, 200, 255);
      for (var y = 0; y < pixels.height; y++) {
        for (var x = 0; x < pixels.width; x++) {
          expect(pixels.pixelAt(x, y), (100, 150, 200, 255));
        }
      }
    });

    test('works on empty buffers and validates channels', () {
      expect(() => PixelContainer(0, 0).fill(1, 2, 3, 4), returnsNormally);
      expect(() => PixelContainer(0, 0).fill(0, 0, 0, 999), throwsRangeError);
    });
  });

  group('value behavior', () {
    test('copy is deeply independent', () {
      final original = PixelContainer(2, 2)..setPixel(0, 0, 1, 2, 3, 4);
      final copied = original.copy();
      copied.setPixel(0, 0, 99, 99, 99, 99);
      expect(original.pixelAt(0, 0), (1, 2, 3, 4));
      expect(copied.pixelAt(0, 0), (99, 99, 99, 99));
    });

    test('equality and hash include dimensions and all bytes', () {
      final a = PixelContainer.fromData(1, 1, Uint8List.fromList([1, 2, 3, 4]));
      final b = PixelContainer.fromData(1, 1, Uint8List.fromList([1, 2, 3, 4]));
      final c = PixelContainer.fromData(1, 1, Uint8List.fromList([1, 2, 3, 5]));
      expect(a == b, isTrue);
      expect(a.hashCode, b.hashCode);
      expect(a == c, isFalse);
      expect(a == PixelContainer(2, 0), isFalse);
    });
  });

  group('top-level parity helpers', () {
    test('factory, access, write, and fill delegate to the class', () {
      final pixels = createPixelContainer(2, 2);
      setPixel(pixels, 1, 0, 10, 20, 30, 40);
      expect(pixelAt(pixels, 1, 0), (10, 20, 30, 40));
      fillPixels(pixels, 9, 8, 7, 6);
      expect(pixelAt(pixels, 0, 1), (9, 8, 7, 6));
    });
  });

  group('ImageCodec contract', () {
    test('exposes MIME type and round-trips pixels', () {
      final codec = StubCodec();
      final original = PixelContainer(2, 1)
        ..setPixel(0, 0, 10, 20, 30, 40)
        ..setPixel(1, 0, 50, 60, 70, 80);
      expect(codec.mimeType, 'image/x-stub');
      expect(codec.decode(codec.encode(original)), original);
    });

    test('implementations can report decode failures', () {
      expect(() => StubCodec().decode(Uint8List(0)), throwsFormatException);
    });
  });
}
