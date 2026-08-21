import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_image_codec_png/coding_adventures_image_codec_png.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';
import 'package:coding_adventures_zip/coding_adventures_zip.dart' as zip;
import 'package:test/test.dart';

void main() {
  test('implements the PixelContainer ImageCodec contract', () {
    final ImageCodec codec = PngCodec();
    expect(codec.mimeType, 'image/png');

    final pixels = PixelContainer.fromData(
      1,
      1,
      Uint8List.fromList([1, 2, 3, 4]),
    );
    expect(codec.decode(codec.encode(pixels)), pixels);
    expect(PngCodec(maxPixels: 1).decode(codec.encode(pixels)), pixels);
  });

  test('validates caller pixel limits without numeric coercion', () {
    final invalid = <num>[
      0,
      -1,
      1.5,
      defaultMaxPixels + 1,
      double.nan,
      double.infinity,
      double.negativeInfinity,
    ];
    for (final value in invalid) {
      _expectCode('invalid-max-pixels', () => PngCodec(maxPixels: value));
      _expectCode(
        'invalid-max-pixels',
        () => decodePng(Uint8List(0), maxPixels: value),
      );
    }
  });

  test('validates typed PixelContainer state before codec allocation', () {
    _expectCode(
      'invalid-image-dimensions',
      () => encodePng(PixelContainer(0, 1)),
    );
    _expectCode(
      'invalid-image-dimensions',
      () => encodePng(PixelContainer(maxDimension + 1, 1)),
    );
    _expectCode(
      'invalid-pixel-data-length',
      () => encodePng(_MalformedPixelContainer(1, 1, Uint8List(3))),
    );
  });

  test('decodes a Uint8List view with a nonzero backing-buffer offset', () {
    final encoded = encodePng(PixelContainer(1, 1));
    final framed = Uint8List(encoded.length + 7);
    framed.setRange(3, 3 + encoded.length, encoded);
    final view = Uint8List.sublistView(framed, 3, 3 + encoded.length);
    expect(decodePng(view), PixelContainer(1, 1));
  });

  test('publishes a closed payload-blind error taxonomy', () {
    expect(pngErrorCodes, hasLength(29));
    expect(() => pngErrorCodes[0] = 'changed', throwsUnsupportedError);
    final error = PngError('invalid-filter');
    expect(error.code, 'invalid-filter');
    expect(error.message, 'invalid-filter');
    expect(error.toString(), isNot(contains('Instance of')));
  });

  test('preserves CRC and first-IHDR precedence for APNG', () {
    final encoded = encodePng(PixelContainer(1, 1));
    final valid = _chunk('acTL', Uint8List(8));
    _expectCode(
      'unsupported-feature',
      () => decodePng(_insert(encoded, 33, valid)),
    );

    final corrupt = Uint8List.fromList(valid)..[valid.length - 1] ^= 1;
    _expectCode(
      'chunk-crc-mismatch',
      () => decodePng(_insert(encoded, 33, corrupt)),
    );
    _expectCode(
      'chunk-before-ihdr',
      () => decodePng(_insert(encoded, 8, valid)),
    );
  });

  test('matches published Adler-32 vectors across the reduction boundary', () {
    expect(adler32(Uint8List.fromList(ascii.encode('Wikipedia'))), 0x11e60398);
    final boundary = Uint8List.fromList(
      List<int>.generate(5553, (i) => i & 0xff),
    );
    expect(adler32(boundary), 0x2ccab2ef);
  });
}

void _expectCode(String expected, void Function() action) {
  try {
    action();
    fail('expected PngError($expected)');
  } on PngError catch (error) {
    expect(error.code, expected);
    expect(error.message, expected);
  }
}

Uint8List _chunk(String type, Uint8List payload) {
  final typeBytes = Uint8List.fromList(ascii.encode(type));
  var checksum = zip.crc32(typeBytes);
  checksum = zip.crc32(payload, checksum);
  final out = BytesBuilder(copy: false)
    ..add(_u32(payload.length))
    ..add(typeBytes)
    ..add(payload)
    ..add(_u32(checksum));
  return out.takeBytes();
}

Uint8List _insert(Uint8List original, int offset, Uint8List inserted) =>
    (BytesBuilder(copy: false)
          ..add(Uint8List.sublistView(original, 0, offset))
          ..add(inserted)
          ..add(Uint8List.sublistView(original, offset)))
        .takeBytes();

Uint8List _u32(int value) => Uint8List.fromList([
      (value >> 24) & 0xff,
      (value >> 16) & 0xff,
      (value >> 8) & 0xff,
      value & 0xff,
    ]);

final class _MalformedPixelContainer extends PixelContainer {
  _MalformedPixelContainer(this._width, this._height, this._data) : super(0, 0);

  final int _width;
  final int _height;
  final Uint8List _data;

  @override
  int get width => _width;

  @override
  int get height => _height;

  @override
  Uint8List get data => _data;
}
