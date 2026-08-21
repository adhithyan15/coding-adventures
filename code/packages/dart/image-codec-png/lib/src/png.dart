import 'dart:typed_data';

import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';
import 'package:coding_adventures_zip/coding_adventures_zip.dart' as zip;

/// Largest width or height accepted by the portable PNG profile.
const int maxDimension = 16384;

/// Default and hard pixel-count ceiling.
const int defaultMaxPixels = 32 * 1024 * 1024;

/// Closed IC18 failure taxonomy in normative order.
final List<String> pngErrorCodes = List<String>.unmodifiable(<String>[
  'invalid-max-pixels',
  'invalid-image-dimensions',
  'invalid-pixel-data-length',
  'file-too-short',
  'invalid-signature',
  'truncated-chunk',
  'invalid-chunk-type',
  'chunk-crc-mismatch',
  'chunk-before-ihdr',
  'duplicate-ihdr',
  'invalid-ihdr-length',
  'invalid-dimensions',
  'dimension-limit',
  'pixel-limit',
  'unsupported-feature',
  'invalid-plte',
  'invalid-trns',
  'nonconsecutive-idat',
  'invalid-iend',
  'trailing-data',
  'unknown-critical-chunk',
  'missing-required-chunk',
  'invalid-zlib-header',
  'preset-dictionary',
  'inflate-failed',
  'inflated-length-mismatch',
  'idat-cavity',
  'adler-mismatch',
  'invalid-filter',
]);

/// Payload-blind portable PNG failure.
final class PngError extends FormatException {
  PngError(this.code) : super(code);

  /// Language-neutral failure identifier.
  final String code;

  @override
  String toString() => code;
}

Never _fail(String code) => throw PngError(code);

int _validateMaxPixels(num? requested) {
  if (requested == null) return defaultMaxPixels;
  final value = requested.toDouble();
  if (!value.isFinite ||
      value != value.truncateToDouble() ||
      value <= 0 ||
      value > defaultMaxPixels) {
    _fail('invalid-max-pixels');
  }
  return value.toInt();
}

/// Stateful [ImageCodec] adapter with an eagerly validated pixel ceiling.
final class PngCodec implements ImageCodec {
  PngCodec({num? maxPixels}) : _maxPixels = _validateMaxPixels(maxPixels);

  final int _maxPixels;

  @override
  String get mimeType => 'image/png';

  @override
  Uint8List encode(PixelContainer pixels) => encodePng(pixels);

  @override
  PixelContainer decode(Uint8List bytes) =>
      _decodePngWithLimit(bytes, _maxPixels);
}

const List<int> _signature = <int>[
  0x89,
  0x50,
  0x4e,
  0x47,
  0x0d,
  0x0a,
  0x1a,
  0x0a,
];

const int _adlerMod = 65521;

/// Compute the RFC 1950 Adler-32 checksum used by PNG's zlib wrapper.
int adler32(Uint8List data) {
  var a = 1;
  var b = 0;
  for (var start = 0; start < data.length; start += 5552) {
    final end = start + 5552 < data.length ? start + 5552 : data.length;
    for (var index = start; index < end; index++) {
      a += data[index];
      b += a;
    }
    a %= _adlerMod;
    b %= _adlerMod;
  }
  return ((b << 16) | a) & 0xffffffff;
}

int _paeth(int a, int b, int c) {
  final prediction = a + b - c;
  final distanceA = (prediction - a).abs();
  final distanceB = (prediction - b).abs();
  final distanceC = (prediction - c).abs();
  if (distanceA <= distanceB && distanceA <= distanceC) return a;
  if (distanceB <= distanceC) return b;
  return c;
}

void _applyFilter(
  int filter,
  Uint8List raw,
  Uint8List prior,
  int bytesPerPixel,
  Uint8List output,
) {
  for (var index = 0; index < raw.length; index++) {
    final left = index >= bytesPerPixel ? raw[index - bytesPerPixel] : 0;
    final above = prior[index];
    final aboveLeft = index >= bytesPerPixel ? prior[index - bytesPerPixel] : 0;
    final predicted = switch (filter) {
      1 => left,
      2 => above,
      3 => (left + above) ~/ 2,
      4 => _paeth(left, above, aboveLeft),
      _ => 0,
    };
    output[index] = (raw[index] - predicted) & 0xff;
  }
}

int _chooseFilter(
  Uint8List raw,
  Uint8List prior,
  int bytesPerPixel,
  Uint8List scratch,
  Uint8List best,
) {
  var bestFilter = 0;
  var bestScore = 0x7fffffffffffffff;
  for (var filter = 0; filter <= 4; filter++) {
    _applyFilter(filter, raw, prior, bytesPerPixel, scratch);
    var score = 0;
    for (final value in scratch) {
      score += value < 128 ? value : 256 - value;
    }
    if (score < bestScore) {
      bestScore = score;
      bestFilter = filter;
      best.setAll(0, scratch);
    }
  }
  return bestFilter;
}

void _undoFilter(
  int filter,
  Uint8List row,
  Uint8List prior,
  int bytesPerPixel,
) {
  switch (filter) {
    case 0:
      return;
    case 1:
      for (var index = bytesPerPixel; index < row.length; index++) {
        row[index] = (row[index] + row[index - bytesPerPixel]) & 0xff;
      }
      return;
    case 2:
      for (var index = 0; index < row.length; index++) {
        row[index] = (row[index] + prior[index]) & 0xff;
      }
      return;
    case 3:
      for (var index = 0; index < row.length; index++) {
        final left = index >= bytesPerPixel ? row[index - bytesPerPixel] : 0;
        row[index] = (row[index] + ((left + prior[index]) ~/ 2)) & 0xff;
      }
      return;
    case 4:
      for (var index = 0; index < row.length; index++) {
        final left = index >= bytesPerPixel ? row[index - bytesPerPixel] : 0;
        final aboveLeft = index >= bytesPerPixel
            ? prior[index - bytesPerPixel]
            : 0;
        row[index] =
            (row[index] + _paeth(left, prior[index], aboveLeft)) & 0xff;
      }
      return;
    default:
      _fail('invalid-filter');
  }
}

Uint8List _u32(int value) {
  final result = Uint8List(4);
  ByteData.sublistView(result).setUint32(0, value, Endian.big);
  return result;
}

void _addChunk(BytesBuilder output, String type, Uint8List data) {
  final typeBytes = Uint8List.fromList(type.codeUnits);
  var checksum = zip.crc32(typeBytes);
  checksum = zip.crc32(data, checksum);
  output
    ..add(_u32(data.length))
    ..add(typeBytes)
    ..add(data)
    ..add(_u32(checksum));
}

/// Encode RGBA8 pixels as deterministic colour-type-6 portable PNG.
Uint8List encodePng(PixelContainer pixels) {
  final width = pixels.width;
  final height = pixels.height;
  if (width <= 0 ||
      height <= 0 ||
      width > maxDimension ||
      height > maxDimension) {
    _fail('invalid-image-dimensions');
  }
  final pixelCount = width * height;
  if (pixelCount > defaultMaxPixels) {
    _fail('invalid-image-dimensions');
  }
  if (pixels.data.length != pixelCount * 4) {
    _fail('invalid-pixel-data-length');
  }

  final output = BytesBuilder(copy: false)..add(_signature);
  final ihdr = Uint8List(13);
  final ihdrView = ByteData.sublistView(ihdr);
  ihdrView
    ..setUint32(0, width, Endian.big)
    ..setUint32(4, height, Endian.big);
  ihdr[8] = 8;
  ihdr[9] = 6;
  _addChunk(output, 'IHDR', ihdr);

  final stride = width * 4;
  final filtered = Uint8List(height * (stride + 1));
  final prior = Uint8List(stride);
  final scratch = Uint8List(stride);
  final best = Uint8List(stride);
  for (var rowIndex = 0; rowIndex < height; rowIndex++) {
    final raw = Uint8List.sublistView(
      pixels.data,
      rowIndex * stride,
      (rowIndex + 1) * stride,
    );
    final destination = rowIndex * (stride + 1);
    filtered[destination] = _chooseFilter(raw, prior, 4, scratch, best);
    filtered.setRange(destination + 1, destination + 1 + stride, best);
    prior.setAll(0, raw);
  }

  final deflated = zip.rawDeflate(filtered);
  final idat = BytesBuilder(copy: false)
    ..add(const <int>[0x78, 0x9c])
    ..add(deflated)
    ..add(_u32(adler32(filtered)));
  _addChunk(output, 'IDAT', idat.takeBytes());
  _addChunk(output, 'IEND', Uint8List(0));
  return output.takeBytes();
}

bool _validChunkType(Uint8List type) {
  if (type.length != 4 || (type[2] & 0x20) != 0) return false;
  for (final value in type) {
    final letter =
        (value >= 0x41 && value <= 0x5a) || (value >= 0x61 && value <= 0x7a);
    if (!letter) return false;
  }
  return true;
}

/// Decode the bounded, non-interlaced, 8-bit IC18 portable PNG profile.
PixelContainer decodePng(Uint8List data, {num? maxPixels}) =>
    _decodePngWithLimit(data, _validateMaxPixels(maxPixels));

PixelContainer _decodePngWithLimit(Uint8List data, int limit) {
  if (data.length < _signature.length) _fail('file-too-short');
  for (var index = 0; index < _signature.length; index++) {
    if (data[index] != _signature[index]) _fail('invalid-signature');
  }

  final input = ByteData.sublistView(data);
  var width = 0;
  var height = 0;
  var colourType = 0;
  var sawIhdr = false;
  var sawIend = false;
  var sawPlte = false;
  var sawTrns = false;
  var inIdat = false;
  var idatEnded = false;
  int? transparentGrey;
  List<int>? transparentRgb;
  final idatParts = <Uint8List>[];

  var position = _signature.length;
  while (position < data.length) {
    if (data.length - position < 8) _fail('truncated-chunk');
    final length = input.getUint32(position, Endian.big);
    if (length > data.length - position - 12) _fail('truncated-chunk');
    final typeStart = position + 4;
    final dataStart = position + 8;
    final dataEnd = dataStart + length;
    final typeBytes = Uint8List.sublistView(data, typeStart, dataStart);
    if (!_validChunkType(typeBytes)) _fail('invalid-chunk-type');
    final declaredCrc = input.getUint32(dataEnd, Endian.big);
    var actualCrc = zip.crc32(typeBytes);
    actualCrc = zip.crc32(
      Uint8List.sublistView(data, dataStart, dataEnd),
      actualCrc,
    );
    if (actualCrc != declaredCrc) _fail('chunk-crc-mismatch');
    final type = String.fromCharCodes(typeBytes);
    final chunkData = Uint8List.sublistView(data, dataStart, dataEnd);
    if (!sawIhdr && type != 'IHDR') _fail('chunk-before-ihdr');

    switch (type) {
      case 'IHDR':
        if (sawIhdr) _fail('duplicate-ihdr');
        if (length != 13) _fail('invalid-ihdr-length');
        final header = ByteData.sublistView(chunkData);
        width = header.getUint32(0, Endian.big);
        height = header.getUint32(4, Endian.big);
        final bitDepth = chunkData[8];
        colourType = chunkData[9];
        if (width == 0 || height == 0) _fail('invalid-dimensions');
        if (width > maxDimension || height > maxDimension) {
          _fail('dimension-limit');
        }
        if (width * height > limit) _fail('pixel-limit');
        if (chunkData[10] != 0 || chunkData[11] != 0 || chunkData[12] != 0) {
          _fail('unsupported-feature');
        }
        if (bitDepth != 8 ||
            colourType == 3 ||
            (colourType != 0 &&
                colourType != 2 &&
                colourType != 4 &&
                colourType != 6)) {
          _fail('unsupported-feature');
        }
        sawIhdr = true;
        break;

      case 'PLTE':
        if (sawPlte ||
            idatParts.isNotEmpty ||
            sawTrns ||
            (colourType != 2 && colourType != 6) ||
            length < 3 ||
            length > 768 ||
            length % 3 != 0) {
          _fail('invalid-plte');
        }
        sawPlte = true;
        break;

      case 'tRNS':
        if (sawTrns || idatParts.isNotEmpty) _fail('invalid-trns');
        final transparency = ByteData.sublistView(chunkData);
        if (colourType == 0) {
          if (length != 2 || transparency.getUint16(0, Endian.big) > 255) {
            _fail('invalid-trns');
          }
          transparentGrey = transparency.getUint16(0, Endian.big);
        } else if (colourType == 2) {
          if (length != 6) _fail('invalid-trns');
          final values = <int>[];
          for (var index = 0; index < 3; index++) {
            final value = transparency.getUint16(index * 2, Endian.big);
            if (value > 255) _fail('invalid-trns');
            values.add(value);
          }
          transparentRgb = values;
        } else {
          _fail('invalid-trns');
        }
        sawTrns = true;
        break;

      case 'IDAT':
        if (idatEnded) _fail('nonconsecutive-idat');
        idatParts.add(chunkData);
        inIdat = true;
        break;

      case 'IEND':
        if (length != 0) _fail('invalid-iend');
        if (dataEnd + 4 != data.length) _fail('trailing-data');
        sawIend = true;
        position = dataEnd + 4;
        continue;

      case 'acTL':
      case 'fcTL':
      case 'fdAT':
        _fail('unsupported-feature');

      default:
        if ((typeBytes[0] & 0x20) == 0) _fail('unknown-critical-chunk');
    }

    if (type != 'IDAT' && inIdat) {
      inIdat = false;
      idatEnded = true;
    }
    position = dataEnd + 4;
  }

  if (!sawIhdr || !sawIend || idatParts.isEmpty) {
    _fail('missing-required-chunk');
  }
  var zlibLength = 0;
  for (final part in idatParts) {
    zlibLength += part.length;
  }
  if (zlibLength > data.length) _fail('truncated-chunk');
  final zlibBuilder = BytesBuilder(copy: false);
  for (final part in idatParts) {
    zlibBuilder.add(part);
  }
  final zlibData = zlibBuilder.takeBytes();
  if (zlibData.length < 6) _fail('invalid-zlib-header');
  final cmf = zlibData[0];
  final flg = zlibData[1];
  if ((cmf & 0x0f) != 8 || (cmf >> 4) > 7 || ((cmf << 8) | flg) % 31 != 0) {
    _fail('invalid-zlib-header');
  }
  if ((flg & 0x20) != 0) _fail('preset-dictionary');

  final channels = switch (colourType) {
    0 => 1,
    2 => 3,
    4 => 2,
    _ => 4,
  };
  final stride = width * channels;
  final expected = height * (stride + 1);
  final deflateData = Uint8List.sublistView(zlibData, 2, zlibData.length - 4);
  late final ({Uint8List output, int bytesConsumed}) inflated;
  try {
    inflated = zip.rawInflateCounted(deflateData, maxOutput: expected);
  } on zip.RawInflateError catch (error) {
    if (error.code == 'output-limit-exceeded') {
      _fail('inflated-length-mismatch');
    }
    _fail('inflate-failed');
  }
  if (inflated.output.length != expected) {
    _fail('inflated-length-mismatch');
  }
  if (inflated.bytesConsumed != deflateData.length) _fail('idat-cavity');
  final declaredAdler = ByteData.sublistView(
    zlibData,
  ).getUint32(zlibData.length - 4, Endian.big);
  if (adler32(inflated.output) != declaredAdler) _fail('adler-mismatch');

  final rowSize = stride + 1;
  for (var rowIndex = 0; rowIndex < height; rowIndex++) {
    if (inflated.output[rowIndex * rowSize] > 4) _fail('invalid-filter');
  }

  final container = PixelContainer(width, height);
  final prior = Uint8List(stride);
  for (var rowIndex = 0; rowIndex < height; rowIndex++) {
    final sourceOffset = rowIndex * rowSize;
    final row = Uint8List.sublistView(
      inflated.output,
      sourceOffset + 1,
      sourceOffset + rowSize,
    );
    _undoFilter(inflated.output[sourceOffset], row, prior, channels);
    final destinationRow = rowIndex * width * 4;
    for (var column = 0; column < width; column++) {
      final source = column * channels;
      final destination = destinationRow + column * 4;
      switch (channels) {
        case 1:
          final value = row[source];
          container.data
            ..[destination] = value
            ..[destination + 1] = value
            ..[destination + 2] = value
            ..[destination + 3] = transparentGrey == value ? 0 : 255;
          break;
        case 2:
          final value = row[source];
          container.data
            ..[destination] = value
            ..[destination + 1] = value
            ..[destination + 2] = value
            ..[destination + 3] = row[source + 1];
          break;
        case 3:
          final red = row[source];
          final green = row[source + 1];
          final blue = row[source + 2];
          final transparent =
              transparentRgb != null &&
              transparentRgb[0] == red &&
              transparentRgb[1] == green &&
              transparentRgb[2] == blue;
          container.data
            ..[destination] = red
            ..[destination + 1] = green
            ..[destination + 2] = blue
            ..[destination + 3] = transparent ? 0 : 255;
          break;
        default:
          container.data.setRange(destination, destination + 4, row, source);
      }
    }
    prior.setAll(0, row);
  }
  return container;
}
