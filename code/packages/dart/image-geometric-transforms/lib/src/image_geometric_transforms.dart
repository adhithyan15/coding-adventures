import 'dart:math' as math;

import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';

/// Sampling kernel for continuous-coordinate image reads.
enum Interpolation { nearest, bilinear, bicubic }

/// Whether arbitrary rotation expands the canvas or preserves its dimensions.
enum RotateBounds { fit, crop }

/// Policy for source coordinates outside the image.
enum OutOfBounds { zero, replicate, reflect, wrap }

/// One red, green, blue, alpha pixel with channels in the range 0 through 255.
typedef Rgba8 = Rgba;

final List<double> _srgbToLinear = List<double>.generate(256, (value) {
  final encoded = value / 255;
  return encoded <= 0.04045
      ? encoded / 12.92
      : math.pow((encoded + 0.055) / 1.055, 2.4).toDouble();
}, growable: false);

double _decode(int value) => _srgbToLinear[value & 0xff];

int _encode(double value) {
  final encoded = value <= 0.0031308
      ? 12.92 * value
      : 1.055 * math.pow(value, 1 / 2.4).toDouble() - 0.055;
  return (encoded.clamp(0.0, 1.0) * 255).round();
}

int? _resolve(int coordinate, int maximum, OutOfBounds policy) {
  if (maximum <= 0) return null;
  if (coordinate >= 0 && coordinate < maximum) return coordinate;
  return switch (policy) {
    OutOfBounds.zero => null,
    OutOfBounds.replicate => coordinate.clamp(0, maximum - 1),
    OutOfBounds.reflect => _reflect(coordinate, maximum),
    OutOfBounds.wrap => ((coordinate % maximum) + maximum) % maximum,
  };
}

int _reflect(int coordinate, int maximum) {
  final period = 2 * maximum;
  var result = ((coordinate % period) + period) % period;
  if (result >= maximum) result = period - 1 - result;
  return result;
}

double _catmullRom(double distance) {
  final value = distance.abs();
  if (value >= 2) return 0;
  if (value >= 1) {
    return -0.5 * value * value * value + 2.5 * value * value - 4 * value + 2;
  }
  return 1.5 * value * value * value - 2.5 * value * value + 1;
}

int _channel(Rgba8 pixel, int index) => switch (index) {
      0 => pixel.$1,
      1 => pixel.$2,
      2 => pixel.$3,
      3 => pixel.$4,
      _ => throw RangeError.index(index, pixel, 'index'),
    };

Rgba8 _read(PixelContainer image, int x, int y, OutOfBounds policy) {
  final resolvedX = _resolve(x, image.width, policy);
  final resolvedY = _resolve(y, image.height, policy);
  if (resolvedX == null || resolvedY == null) return transparent;
  return image.pixelAt(resolvedX, resolvedY);
}

Rgba8 _sampleNearest(
  PixelContainer image,
  double u,
  double v,
  OutOfBounds policy,
) =>
    _read(image, u.round(), v.round(), policy);

Rgba8 _sampleBilinear(
  PixelContainer image,
  double u,
  double v,
  OutOfBounds policy,
) {
  final x0 = u.floor();
  final y0 = v.floor();
  final wx1 = u - x0;
  final wx0 = 1 - wx1;
  final wy1 = v - y0;
  final wy0 = 1 - wy1;
  final p00 = _read(image, x0, y0, policy);
  final p10 = _read(image, x0 + 1, y0, policy);
  final p01 = _read(image, x0, y0 + 1, policy);
  final p11 = _read(image, x0 + 1, y0 + 1, policy);

  int blendRgb(int channel) {
    final linear = _decode(_channel(p00, channel)) * wx0 * wy0 +
        _decode(_channel(p10, channel)) * wx1 * wy0 +
        _decode(_channel(p01, channel)) * wx0 * wy1 +
        _decode(_channel(p11, channel)) * wx1 * wy1;
    return _encode(linear);
  }

  final alpha = (p00.$4 * wx0 * wy0 +
          p10.$4 * wx1 * wy0 +
          p01.$4 * wx0 * wy1 +
          p11.$4 * wx1 * wy1)
      .round();
  return (blendRgb(0), blendRgb(1), blendRgb(2), alpha);
}

Rgba8 _sampleBicubic(
  PixelContainer image,
  double u,
  double v,
  OutOfBounds policy,
) {
  final x0 = u.floor();
  final y0 = v.floor();
  final horizontal = [
    for (var offset = -1; offset <= 2; offset++) _catmullRom(u - (x0 + offset)),
  ];
  final vertical = [
    for (var offset = -1; offset <= 2; offset++) _catmullRom(v - (y0 + offset)),
  ];
  final accumulator = List<double>.filled(4, 0);

  for (var dy = -1; dy <= 2; dy++) {
    for (var dx = -1; dx <= 2; dx++) {
      final pixel = _read(image, x0 + dx, y0 + dy, policy);
      final weight = horizontal[dx + 1] * vertical[dy + 1];
      accumulator[0] += _decode(pixel.$1) * weight;
      accumulator[1] += _decode(pixel.$2) * weight;
      accumulator[2] += _decode(pixel.$3) * weight;
      accumulator[3] += pixel.$4 / 255 * weight;
    }
  }
  return (
    _encode(accumulator[0]),
    _encode(accumulator[1]),
    _encode(accumulator[2]),
    (accumulator[3] * 255).clamp(0.0, 255.0).round(),
  );
}

/// Sample [image] at a continuous coordinate using [interpolation] and [oob].
Rgba8 sample(
  PixelContainer image,
  num u,
  num v,
  Interpolation interpolation,
  OutOfBounds oob,
) {
  final sourceU = u.toDouble();
  final sourceV = v.toDouble();
  if (!sourceU.isFinite || !sourceV.isFinite) {
    throw ArgumentError('sample coordinates must be finite');
  }
  return switch (interpolation) {
    Interpolation.nearest => _sampleNearest(image, sourceU, sourceV, oob),
    Interpolation.bilinear => _sampleBilinear(image, sourceU, sourceV, oob),
    Interpolation.bicubic => _sampleBicubic(image, sourceU, sourceV, oob),
  };
}

void _write(PixelContainer image, int x, int y, Rgba8 pixel) =>
    image.setPixel(x, y, pixel.$1, pixel.$2, pixel.$3, pixel.$4);

/// Mirror [source] from left to right without interpolation.
PixelContainer flipHorizontal(PixelContainer source) {
  final output = PixelContainer(source.width, source.height);
  for (var y = 0; y < source.height; y++) {
    for (var x = 0; x < source.width; x++) {
      _write(output, source.width - 1 - x, y, source.pixelAt(x, y));
    }
  }
  return output;
}

/// Mirror [source] from top to bottom without interpolation.
PixelContainer flipVertical(PixelContainer source) {
  final output = PixelContainer(source.width, source.height);
  for (var y = 0; y < source.height; y++) {
    for (var x = 0; x < source.width; x++) {
      _write(output, x, source.height - 1 - y, source.pixelAt(x, y));
    }
  }
  return output;
}

/// Rotate [source] 90 degrees clockwise without interpolation.
PixelContainer rotate90CW(PixelContainer source) {
  final output = PixelContainer(source.height, source.width);
  for (var y = 0; y < output.height; y++) {
    for (var x = 0; x < output.width; x++) {
      _write(output, x, y, source.pixelAt(y, source.height - 1 - x));
    }
  }
  return output;
}

/// Rotate [source] 90 degrees counter-clockwise without interpolation.
PixelContainer rotate90CCW(PixelContainer source) {
  final output = PixelContainer(source.height, source.width);
  for (var y = 0; y < output.height; y++) {
    for (var x = 0; x < output.width; x++) {
      _write(output, x, y, source.pixelAt(source.width - 1 - y, x));
    }
  }
  return output;
}

/// Rotate [source] 180 degrees without interpolation.
PixelContainer rotate180(PixelContainer source) {
  final output = PixelContainer(source.width, source.height);
  for (var y = 0; y < source.height; y++) {
    for (var x = 0; x < source.width; x++) {
      _write(
        output,
        x,
        y,
        source.pixelAt(source.width - 1 - x, source.height - 1 - y),
      );
    }
  }
  return output;
}

/// Extract a [width] by [height] region starting at ([x0], [y0]).
PixelContainer crop(
  PixelContainer source,
  int x0,
  int y0,
  int width,
  int height,
) {
  final output = PixelContainer(width, height);
  for (var y = 0; y < height; y++) {
    for (var x = 0; x < width; x++) {
      _write(output, x, y, source.pixelAt(x0 + x, y0 + y));
    }
  }
  return output;
}

/// Add solid borders around [source].
PixelContainer pad(
  PixelContainer source,
  int top,
  int right,
  int bottom,
  int left,
  Rgba8 fill,
) {
  for (final entry in {
    'top': top,
    'right': right,
    'bottom': bottom,
    'left': left,
  }.entries) {
    if (entry.value < 0) {
      throw RangeError.value(entry.value, entry.key, 'must be non-negative');
    }
  }
  final output = PixelContainer(
    left + source.width + right,
    top + source.height + bottom,
  )..fill(fill.$1, fill.$2, fill.$3, fill.$4);
  for (var y = 0; y < source.height; y++) {
    for (var x = 0; x < source.width; x++) {
      _write(output, left + x, top + y, source.pixelAt(x, y));
    }
  }
  return output;
}

void _requireNonEmpty(PixelContainer source) {
  if (source.width == 0 || source.height == 0) {
    throw StateError('continuous transforms require a non-empty source image');
  }
}

/// Resize [source] with pixel-centre alignment and replicated borders.
PixelContainer scale(
  PixelContainer source,
  int outputWidth,
  int outputHeight, {
  Interpolation interpolation = Interpolation.bilinear,
}) {
  _requireNonEmpty(source);
  final output = PixelContainer(outputWidth, outputHeight);
  final scaleX = outputWidth / source.width;
  final scaleY = outputHeight / source.height;
  for (var y = 0; y < outputHeight; y++) {
    for (var x = 0; x < outputWidth; x++) {
      final u = (x + 0.5) / scaleX - 0.5;
      final v = (y + 0.5) / scaleY - 0.5;
      _write(
        output,
        x,
        y,
        sample(source, u, v, interpolation, OutOfBounds.replicate),
      );
    }
  }
  return output;
}

/// Rotate [source] counter-clockwise by [radians] around its centre.
PixelContainer rotate(
  PixelContainer source,
  num radians, {
  Interpolation interpolation = Interpolation.bilinear,
  RotateBounds bounds = RotateBounds.fit,
}) {
  _requireNonEmpty(source);
  final angle = radians.toDouble();
  if (!angle.isFinite) throw ArgumentError.value(radians, 'radians');
  final cosine = math.cos(angle);
  final sine = math.sin(angle);
  final outputWidth = bounds == RotateBounds.fit
      ? (source.width * cosine.abs() + source.height * sine.abs()).ceil()
      : source.width;
  final outputHeight = bounds == RotateBounds.fit
      ? (source.width * sine.abs() + source.height * cosine.abs()).ceil()
      : source.height;
  final centerXIn = source.width / 2;
  final centerYIn = source.height / 2;
  final centerXOut = outputWidth / 2;
  final centerYOut = outputHeight / 2;
  final output = PixelContainer(outputWidth, outputHeight);
  for (var y = 0; y < outputHeight; y++) {
    for (var x = 0; x < outputWidth; x++) {
      final dx = x - centerXOut;
      final dy = y - centerYOut;
      final u = centerXIn + cosine * dx + sine * dy;
      final v = centerYIn - sine * dx + cosine * dy;
      _write(
        output,
        x,
        y,
        sample(source, u, v, interpolation, OutOfBounds.zero),
      );
    }
  }
  return output;
}

void _validateMatrix(
  List<List<num>> matrix,
  int rows,
  int columns,
  String name,
) {
  if (matrix.length != rows || matrix.any((row) => row.length != columns)) {
    throw ArgumentError.value(matrix, name, 'must be ${rows}x$columns');
  }
  if (matrix.expand((row) => row).any((value) => !value.isFinite)) {
    throw ArgumentError.value(matrix, name, 'entries must be finite');
  }
}

/// Apply a 2x3 inverse affine mapping to [source].
PixelContainer affine(
  PixelContainer source,
  List<List<num>> matrix,
  int outputWidth,
  int outputHeight, {
  Interpolation interpolation = Interpolation.bilinear,
  OutOfBounds oob = OutOfBounds.zero,
}) {
  _requireNonEmpty(source);
  _validateMatrix(matrix, 2, 3, 'matrix');
  final output = PixelContainer(outputWidth, outputHeight);
  for (var y = 0; y < outputHeight; y++) {
    for (var x = 0; x < outputWidth; x++) {
      final u = matrix[0][0] * x + matrix[0][1] * y + matrix[0][2];
      final v = matrix[1][0] * x + matrix[1][1] * y + matrix[1][2];
      _write(output, x, y, sample(source, u, v, interpolation, oob));
    }
  }
  return output;
}

/// Apply a 3x3 inverse homography to [source].
PixelContainer perspectiveWarp(
  PixelContainer source,
  List<List<num>> homography,
  int outputWidth,
  int outputHeight, {
  Interpolation interpolation = Interpolation.bilinear,
  OutOfBounds oob = OutOfBounds.zero,
}) {
  _requireNonEmpty(source);
  _validateMatrix(homography, 3, 3, 'homography');
  final output = PixelContainer(outputWidth, outputHeight);
  for (var y = 0; y < outputHeight; y++) {
    for (var x = 0; x < outputWidth; x++) {
      final uHomogeneous =
          homography[0][0] * x + homography[0][1] * y + homography[0][2];
      final vHomogeneous =
          homography[1][0] * x + homography[1][1] * y + homography[1][2];
      final weight =
          homography[2][0] * x + homography[2][1] * y + homography[2][2];
      if (weight == 0) continue;
      _write(
        output,
        x,
        y,
        sample(
          source,
          uHomogeneous / weight,
          vHomogeneous / weight,
          interpolation,
          oob,
        ),
      );
    }
  }
  return output;
}
