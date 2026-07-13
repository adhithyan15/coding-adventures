import 'dart:typed_data';

/// Package semantic version.
const String version = '0.1.0';

/// One red, green, blue, alpha pixel in that order.
typedef Rgba = (int, int, int, int);

/// Fully transparent black, returned for out-of-bounds reads.
const Rgba transparent = (0, 0, 0, 0);

/// Contract implemented by image encoders and decoders.
abstract interface class ImageCodec {
  /// IANA MIME type for this image format.
  String get mimeType;

  /// Encode [pixels] as a complete image file.
  Uint8List encode(PixelContainer pixels);

  /// Decode a complete image file into RGBA8 pixels.
  PixelContainer decode(Uint8List bytes);
}

/// A flat, row-major RGBA8 pixel buffer with a top-left origin.
///
/// Each pixel occupies four bytes in [data]: red, green, blue, then alpha.
/// The byte offset for `(x, y)` is `(y * width + x) * 4`.
class PixelContainer {
  /// Create a transparent, zero-filled buffer.
  PixelContainer(int width, int height)
      : width = width,
        height = height,
        data = Uint8List(_checkedByteCount(width, height));

  PixelContainer._(this.width, this.height, this.data);

  /// Create a buffer by copying existing RGBA8 [data].
  factory PixelContainer.fromData(int width, int height, Uint8List data) {
    final expected = _checkedByteCount(width, height);
    if (data.length != expected) {
      throw ArgumentError.value(
        data.length,
        'data.length',
        'expected $expected bytes for ${width}x$height RGBA8 pixels',
      );
    }
    return PixelContainer._(width, height, Uint8List.fromList(data));
  }

  /// Width in pixels.
  final int width;

  /// Height in pixels.
  final int height;

  /// Mutable RGBA8 bytes in row-major order.
  final Uint8List data;

  /// Number of pixels in the buffer.
  int get pixelCount => width * height;

  /// Number of bytes in the backing buffer.
  int get byteCount => data.length;

  /// Read `(r, g, b, a)` at ([x], [y]), or [transparent] when out of bounds.
  Rgba pixelAt(int x, int y) {
    if (!_contains(x, y)) return transparent;
    final offset = _offset(x, y);
    return (data[offset], data[offset + 1], data[offset + 2], data[offset + 3]);
  }

  /// Write an RGBA8 pixel. Out-of-bounds coordinates are a no-op.
  void setPixel(int x, int y, int red, int green, int blue, int alpha) {
    if (!_contains(x, y)) return;
    _checkChannel(red, 'red');
    _checkChannel(green, 'green');
    _checkChannel(blue, 'blue');
    _checkChannel(alpha, 'alpha');
    final offset = _offset(x, y);
    data[offset] = red;
    data[offset + 1] = green;
    data[offset + 2] = blue;
    data[offset + 3] = alpha;
  }

  /// Fill every pixel with one RGBA8 colour.
  void fill(int red, int green, int blue, int alpha) {
    _checkChannel(red, 'red');
    _checkChannel(green, 'green');
    _checkChannel(blue, 'blue');
    _checkChannel(alpha, 'alpha');
    for (var offset = 0; offset < data.length; offset += 4) {
      data[offset] = red;
      data[offset + 1] = green;
      data[offset + 2] = blue;
      data[offset + 3] = alpha;
    }
  }

  /// Create an independent copy, including its pixel bytes.
  PixelContainer copy() => PixelContainer.fromData(width, height, data);

  bool _contains(int x, int y) => x >= 0 && y >= 0 && x < width && y < height;

  int _offset(int x, int y) => (y * width + x) * 4;

  static int _checkedByteCount(int width, int height) {
    if (width < 0) {
      throw RangeError.value(width, 'width', 'must be non-negative');
    }
    if (height < 0) {
      throw RangeError.value(height, 'height', 'must be non-negative');
    }
    final count = BigInt.from(width) * BigInt.from(height) * BigInt.from(4);
    final maxInt = BigInt.parse('9223372036854775807');
    if (count > maxInt) {
      throw RangeError('pixel dimensions exceed addressable memory');
    }
    return count.toInt();
  }

  static void _checkChannel(int value, String name) {
    if (value < 0 || value > 255) {
      throw RangeError.range(value, 0, 255, name);
    }
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! PixelContainer ||
        width != other.width ||
        height != other.height ||
        data.length != other.data.length) {
      return false;
    }
    for (var index = 0; index < data.length; index++) {
      if (data[index] != other.data[index]) return false;
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(width, height, Object.hashAll(data));

  @override
  String toString() =>
      'PixelContainer(width: $width, height: $height, bytes: ${data.length})';
}

/// Top-level factory matching the TypeScript port.
PixelContainer createPixelContainer(int width, int height) =>
    PixelContainer(width, height);

/// Top-level read helper matching the TypeScript port.
Rgba pixelAt(PixelContainer container, int x, int y) => container.pixelAt(x, y);

/// Top-level write helper matching the TypeScript port.
void setPixel(
  PixelContainer container,
  int x,
  int y,
  int red,
  int green,
  int blue,
  int alpha,
) =>
    container.setPixel(x, y, red, green, blue, alpha);

/// Top-level fill helper matching the TypeScript port.
void fillPixels(
  PixelContainer container,
  int red,
  int green,
  int blue,
  int alpha,
) =>
    container.fill(red, green, blue, alpha);
