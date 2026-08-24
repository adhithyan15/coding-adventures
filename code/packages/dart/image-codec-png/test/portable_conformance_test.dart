import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_image_codec_png/coding_adventures_image_codec_png.dart';
import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';
import 'package:image/image.dart' as image;
import 'package:test/test.dart';

void main() {
  final document =
      jsonDecode(_fixtureFile().readAsStringSync()) as Map<String, dynamic>;
  final cases = document['cases'] as List<dynamic>;

  test('pins the complete IC18 fixture identity and public contract', () {
    expect(document['schema_version'], 1);
    expect(document['profile'], 'image-codec-png-v1');
    expect(cases, hasLength(85));
    expect(document['limits'], {
      'max_dimension': maxDimension,
      'default_max_pixels': defaultMaxPixels,
    });
    expect(document['error_ids'], pngErrorCodes);
  });

  for (final value in cases) {
    final fixture = value as Map<String, dynamic>;
    final id = fixture['id'] as String;
    test('consumes $id through public APIs', () {
      switch (fixture['operation']) {
        case 'decode':
          _assertDecode(fixture);
          return;
        case 'decode-error':
          _assertError(fixture, () => _decodeFixture(fixture));
          return;
        case 'encode':
          _assertEncode(fixture);
          return;
        case 'encode-error':
          _assertError(
            fixture,
            () => _encodeFixture(fixture['input'] as Map<String, dynamic>),
          );
          return;
        case 'adler32':
          final actual = adler32(_hex(fixture['input_hex'] as String));
          expect(
            actual.toRadixString(16).padLeft(8, '0'),
            (fixture['expected'] as Map<String, dynamic>)['adler32_hex'],
          );
          return;
        default:
          fail('unknown operation for $id');
      }
    });
  }
}

void _assertDecode(Map<String, dynamic> fixture) {
  final actual = _decodeFixture(fixture);
  final expected = fixture['expected'] as Map<String, dynamic>;
  expect(actual.width, expected['width']);
  expect(actual.height, expected['height']);
  expect(actual.data, _hex(expected['rgba_hex'] as String));
}

PixelContainer _decodeFixture(Map<String, dynamic> fixture) {
  final png = _hex(fixture['png_hex'] as String);
  final options = fixture['options'] as Map<String, dynamic>?;
  return decodePng(png, maxPixels: options?['max_pixels'] as num?);
}

void _assertEncode(Map<String, dynamic> fixture) {
  final input = fixture['input'] as Map<String, dynamic>;
  final encoded = _encodeFixture(input);
  final expected = fixture['expected'] as Map<String, dynamic>;
  final chunks = _parseChunks(encoded);
  expect(chunks.map((chunk) => chunk.type).toList(), expected['chunk_types']);
  expect(encoded[24], expected['bit_depth']);
  expect(encoded[25], expected['colour_type']);
  expect(encoded[28], expected['interlace']);

  final idat = BytesBuilder(copy: false);
  for (final chunk in chunks) {
    if (chunk.type == 'IDAT') idat.add(chunk.data);
  }
  final filtered = ZLibDecoder().convert(idat.takeBytes());
  final width = _exactDimension(input['width']);
  final height = _exactDimension(input['height']);
  final rowSize = width * 4 + 1;
  expect(
    List<int>.generate(height, (row) => filtered[row * rowSize]),
    expected['filter_types'],
  );

  final roundTrip = decodePng(encoded);
  expect(roundTrip.width, width);
  expect(roundTrip.height, height);
  expect(roundTrip.data, _hex(input['rgba_hex'] as String));

  final foreign = image.decodePng(encoded);
  expect(foreign, isNotNull);
  expect(foreign!.width, width);
  expect(foreign.height, height);
  final foreignRgba = Uint8List(width * height * 4);
  var offset = 0;
  for (var y = 0; y < height; y++) {
    for (var x = 0; x < width; x++) {
      final pixel = foreign.getPixel(x, y);
      foreignRgba[offset++] = pixel.r.toInt();
      foreignRgba[offset++] = pixel.g.toInt();
      foreignRgba[offset++] = pixel.b.toInt();
      foreignRgba[offset++] = pixel.a.toInt();
    }
  }
  expect(foreignRgba, _hex(input['rgba_hex'] as String));
}

Uint8List _encodeFixture(Map<String, dynamic> input) {
  final width = _exactDimension(input['width']);
  final height = _exactDimension(input['height']);
  final rgba = _hex(input['rgba_hex'] as String);
  final expectedLength =
      BigInt.from(width) * BigInt.from(height) * BigInt.from(4);
  if (expectedLength != BigInt.from(rgba.length)) {
    throw PngError('invalid-pixel-data-length');
  }
  return encodePng(PixelContainer.fromData(width, height, rgba));
}

int _exactDimension(Object? raw) {
  if (raw is! num) throw PngError('invalid-image-dimensions');
  final value = raw.toDouble();
  if (!value.isFinite || value != value.truncateToDouble()) {
    throw PngError('invalid-image-dimensions');
  }
  if (value < 0 || value > 0x7fffffffffffffff) {
    throw PngError('invalid-image-dimensions');
  }
  return value.toInt();
}

void _assertError(Map<String, dynamic> fixture, void Function() action) {
  final expected = (fixture['expected'] as Map<String, dynamic>)['error_id'];
  try {
    action();
    fail('expected PngError($expected)');
  } on PngError catch (error) {
    expect(error.code, expected);
    expect(error.message, expected);
  }
}

List<_Chunk> _parseChunks(Uint8List png) {
  final chunks = <_Chunk>[];
  var offset = 8;
  while (offset < png.length) {
    final length = _readU32(png, offset);
    final end = offset + 12 + length;
    expect(end, lessThanOrEqualTo(png.length));
    chunks.add(
      _Chunk(
        ascii.decode(Uint8List.sublistView(png, offset + 4, offset + 8)),
        Uint8List.fromList(
          Uint8List.sublistView(png, offset + 8, offset + 8 + length),
        ),
      ),
    );
    offset = end;
  }
  return chunks;
}

int _readU32(Uint8List data, int offset) =>
    data[offset] * 0x1000000 +
    data[offset + 1] * 0x10000 +
    data[offset + 2] * 0x100 +
    data[offset + 3];

Uint8List _hex(String value) {
  final out = Uint8List(value.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(value.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

File _fixtureFile() {
  var current = Directory.current.absolute;
  while (true) {
    final candidate = File(
      '${current.path}${Platform.pathSeparator}code${Platform.pathSeparator}specs'
      '${Platform.pathSeparator}fixtures${Platform.pathSeparator}image-codec-png-v1'
      '${Platform.pathSeparator}cases.json',
    );
    if (candidate.existsSync()) return candidate;
    final parent = current.parent;
    if (parent.path == current.path) {
      throw StateError('could not locate IC18 portable fixture corpus');
    }
    current = parent;
  }
}

final class _Chunk {
  const _Chunk(this.type, this.data);

  final String type;
  final Uint8List data;
}
