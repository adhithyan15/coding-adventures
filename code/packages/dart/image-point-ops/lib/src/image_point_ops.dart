import 'dart:math' as math;
import 'dart:typed_data';

import 'package:coding_adventures_pixel_container/coding_adventures_pixel_container.dart';

/// Package semantic version.
const String version = '0.1.0';

/// Luminance weighting used by [greyscale].
enum GreyscaleMethod {
  /// Rec. 709 weights for modern sRGB displays.
  rec709,

  /// BT. 601 weights for legacy standard-definition video.
  bt601,

  /// Equal red, green, and blue weights.
  average,
}

typedef _PixelMap = Rgba Function(int red, int green, int blue, int alpha);

final List<double> _srgbToLinear = List<double>.generate(256, (value) {
  final channel = value / 255;
  return channel <= 0.04045
      ? channel / 12.92
      : math.pow((channel + 0.055) / 1.055, 2.4).toDouble();
}, growable: false);

double _decode(int value) => _srgbToLinear[value];

int _encode(num linear) {
  final encoded = linear <= 0.0031308
      ? linear * 12.92
      : 1.055 * math.pow(linear, 1 / 2.4) - 0.055;
  return (encoded.clamp(0.0, 1.0) * 255).round();
}

PixelContainer _mapPixels(PixelContainer source, _PixelMap transform) {
  final output = PixelContainer(source.width, source.height);
  for (var offset = 0; offset < source.data.length; offset += 4) {
    final (red, green, blue, alpha) = transform(
      source.data[offset],
      source.data[offset + 1],
      source.data[offset + 2],
      source.data[offset + 3],
    );
    output.data[offset] = red;
    output.data[offset + 1] = green;
    output.data[offset + 2] = blue;
    output.data[offset + 3] = alpha;
  }
  return output;
}

/// Invert every RGB byte while preserving alpha.
PixelContainer invert(PixelContainer source) => _mapPixels(
      source,
      (red, green, blue, alpha) => (255 - red, 255 - green, 255 - blue, alpha),
    );

/// Binarize using the arithmetic mean of the three RGB bytes.
PixelContainer threshold(PixelContainer source, num value) =>
    _mapPixels(source, (red, green, blue, alpha) {
      final output = (red + green + blue) / 3 >= value ? 255 : 0;
      return (output, output, output, alpha);
    });

/// Binarize using Rec. 709 luma over the encoded RGB bytes.
PixelContainer thresholdLuminance(PixelContainer source, num value) =>
    _mapPixels(source, (red, green, blue, alpha) {
      final luma = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
      final output = luma >= value ? 255 : 0;
      return (output, output, output, alpha);
    });

/// Quantize RGB into [levels] equally spaced values.
PixelContainer posterize(PixelContainer source, int levels) {
  if (levels < 2) {
    throw RangeError.range(levels, 2, null, 'levels');
  }
  final step = 255 / (levels - 1);
  int quantize(int value) =>
      ((value / step).round() * step).round().clamp(0, 255).toInt();
  return _mapPixels(
    source,
    (red, green, blue, alpha) =>
        (quantize(red), quantize(green), quantize(blue), alpha),
  );
}

/// Swap the red and blue channels.
PixelContainer swapRgbBgr(PixelContainer source) =>
    _mapPixels(source, (red, green, blue, alpha) => (blue, green, red, alpha));

/// Retain one channel (`0=R`, `1=G`, `2=B`, `3=A`).
///
/// RGB extraction zeroes the other two colour channels and preserves alpha.
/// Alpha extraction leaves the RGB bytes unchanged, matching the shared API.
PixelContainer extractChannel(PixelContainer source, int channel) {
  if (channel < 0 || channel > 3) {
    throw RangeError.range(channel, 0, 3, 'channel');
  }
  return _mapPixels(source, (red, green, blue, alpha) {
    return switch (channel) {
      0 => (red, 0, 0, alpha),
      1 => (0, green, 0, alpha),
      2 => (0, 0, blue, alpha),
      _ => (red, green, blue, alpha),
    };
  });
}

/// Add [offset] to each RGB byte and clamp to the byte range.
PixelContainer brightness(PixelContainer source, num offset) => _mapPixels(
      source,
      (red, green, blue, alpha) => (
        (red + offset).round().clamp(0, 255),
        (green + offset).round().clamp(0, 255),
        (blue + offset).round().clamp(0, 255),
        alpha,
      ),
    );

/// Scale decoded linear-light RGB around 0.5.
PixelContainer contrast(PixelContainer source, num factor) => _mapPixels(
      source,
      (red, green, blue, alpha) => (
        _encode(0.5 + factor * (_decode(red) - 0.5)),
        _encode(0.5 + factor * (_decode(green) - 0.5)),
        _encode(0.5 + factor * (_decode(blue) - 0.5)),
        alpha,
      ),
    );

/// Raise decoded linear-light RGB to [exponent].
PixelContainer gamma(PixelContainer source, num exponent) => _mapPixels(
      source,
      (red, green, blue, alpha) => (
        _encode(math.pow(_decode(red), exponent).toDouble()),
        _encode(math.pow(_decode(green), exponent).toDouble()),
        _encode(math.pow(_decode(blue), exponent).toDouble()),
        alpha,
      ),
    );

/// Multiply decoded linear-light RGB by `2 ^ stops`.
PixelContainer exposure(PixelContainer source, num stops) {
  final factor = math.pow(2, stops).toDouble();
  return _mapPixels(
    source,
    (red, green, blue, alpha) => (
      _encode(_decode(red) * factor),
      _encode(_decode(green) * factor),
      _encode(_decode(blue) * factor),
      alpha,
    ),
  );
}

/// Convert RGB to linear-light luminance, then encode it as sRGB.
PixelContainer greyscale(
  PixelContainer source, [
  GreyscaleMethod method = GreyscaleMethod.rec709,
]) =>
    _mapPixels(source, (red, green, blue, alpha) {
      final linearRed = _decode(red);
      final linearGreen = _decode(green);
      final linearBlue = _decode(blue);
      final luminance = switch (method) {
        GreyscaleMethod.rec709 =>
          0.2126 * linearRed + 0.7152 * linearGreen + 0.0722 * linearBlue,
        GreyscaleMethod.bt601 =>
          0.2989 * linearRed + 0.5870 * linearGreen + 0.1140 * linearBlue,
        GreyscaleMethod.average => (linearRed + linearGreen + linearBlue) / 3,
      };
      final output = _encode(luminance);
      return (output, output, output, alpha);
    });

/// Apply the classic sepia matrix in linear light.
PixelContainer sepia(PixelContainer source) =>
    _mapPixels(source, (red, green, blue, alpha) {
      final linearRed = _decode(red);
      final linearGreen = _decode(green);
      final linearBlue = _decode(blue);
      return (
        _encode(0.393 * linearRed + 0.769 * linearGreen + 0.189 * linearBlue),
        _encode(0.349 * linearRed + 0.686 * linearGreen + 0.168 * linearBlue),
        _encode(0.272 * linearRed + 0.534 * linearGreen + 0.131 * linearBlue),
        alpha,
      );
    });

/// Multiply decoded linear RGB by a row-major 3-by-3 [matrix].
PixelContainer colourMatrix(PixelContainer source, List<List<num>> matrix) {
  if (matrix.length != 3 || matrix.any((row) => row.length != 3)) {
    throw ArgumentError.value(matrix, 'matrix', 'must be a 3-by-3 matrix');
  }
  return _mapPixels(source, (red, green, blue, alpha) {
    final channels = [_decode(red), _decode(green), _decode(blue)];
    double multiplyRow(int row) => (matrix[row][0] * channels[0] +
            matrix[row][1] * channels[1] +
            matrix[row][2] * channels[2])
        .toDouble();
    return (
      _encode(multiplyRow(0)),
      _encode(multiplyRow(1)),
      _encode(multiplyRow(2)),
      alpha,
    );
  });
}

/// Scale saturation around Rec. 709 linear-light luminance.
PixelContainer saturate(PixelContainer source, num factor) =>
    _mapPixels(source, (red, green, blue, alpha) {
      final linearRed = _decode(red);
      final linearGreen = _decode(green);
      final linearBlue = _decode(blue);
      final grey =
          0.2126 * linearRed + 0.7152 * linearGreen + 0.0722 * linearBlue;
      return (
        _encode(grey + factor * (linearRed - grey)),
        _encode(grey + factor * (linearGreen - grey)),
        _encode(grey + factor * (linearBlue - grey)),
        alpha,
      );
    });

(double, double, double) _rgbToHsv(double red, double green, double blue) {
  final maximum = math.max(red, math.max(green, blue));
  final minimum = math.min(red, math.min(green, blue));
  final delta = maximum - minimum;
  final saturation = maximum == 0 ? 0.0 : delta / maximum;
  var hue = 0.0;
  if (delta != 0) {
    if (maximum == red) {
      hue = ((green - blue) / delta) % 6;
    } else if (maximum == green) {
      hue = (blue - red) / delta + 2;
    } else {
      hue = (red - green) / delta + 4;
    }
    hue = (hue * 60 + 360) % 360;
  }
  return (hue, saturation, maximum);
}

(double, double, double) _hsvToRgb(
  double hue,
  double saturation,
  double value,
) {
  final chroma = value * saturation;
  final intermediate = chroma * (1 - ((hue / 60) % 2 - 1).abs());
  final match = value - chroma;
  var red = 0.0;
  var green = 0.0;
  var blue = 0.0;
  if (hue < 60) {
    red = chroma;
    green = intermediate;
  } else if (hue < 120) {
    red = intermediate;
    green = chroma;
  } else if (hue < 180) {
    green = chroma;
    blue = intermediate;
  } else if (hue < 240) {
    green = intermediate;
    blue = chroma;
  } else if (hue < 300) {
    red = intermediate;
    blue = chroma;
  } else {
    red = chroma;
    blue = intermediate;
  }
  return (red + match, green + match, blue + match);
}

/// Rotate linear-light HSV hue by [degrees].
PixelContainer hueRotate(PixelContainer source, num degrees) =>
    _mapPixels(source, (red, green, blue, alpha) {
      final (hue, saturation, value) = _rgbToHsv(
        _decode(red),
        _decode(green),
        _decode(blue),
      );
      final rotatedHue = (hue + degrees) % 360;
      final (newRed, newGreen, newBlue) = _hsvToRgb(
        rotatedHue.toDouble(),
        saturation,
        value,
      );
      return (_encode(newRed), _encode(newGreen), _encode(newBlue), alpha);
    });

/// Convert encoded sRGB bytes to linear-light bytes.
PixelContainer srgbToLinearImage(PixelContainer source) => _mapPixels(
      source,
      (red, green, blue, alpha) => (
        (_decode(red) * 255).round(),
        (_decode(green) * 255).round(),
        (_decode(blue) * 255).round(),
        alpha,
      ),
    );

/// Convert linear-light bytes to encoded sRGB bytes.
PixelContainer linearToSrgbImage(PixelContainer source) => _mapPixels(
      source,
      (red, green, blue, alpha) => (
        _encode(red / 255),
        _encode(green / 255),
        _encode(blue / 255),
        alpha
      ),
    );

/// Apply one 256-entry byte LUT to each RGB channel.
PixelContainer applyLut1dU8(
  PixelContainer source,
  Uint8List redLut,
  Uint8List greenLut,
  Uint8List blueLut,
) {
  for (final (name, lut) in [
    ('redLut', redLut),
    ('greenLut', greenLut),
    ('blueLut', blueLut),
  ]) {
    if (lut.length != 256) {
      throw ArgumentError.value(lut.length, name, 'must contain 256 entries');
    }
  }
  return _mapPixels(
    source,
    (red, green, blue, alpha) =>
        (redLut[red], greenLut[green], blueLut[blue], alpha),
  );
}

/// Sample a linear-light mapping function into a 256-entry sRGB byte LUT.
Uint8List buildLut1dU8(double Function(double linearInput) transform) =>
    Uint8List.fromList([
      for (var value = 0; value < 256; value++)
        _encode(transform(_decode(value))),
    ]);

/// Build the LUT equivalent of [gamma].
Uint8List buildGammaLut(num exponent) =>
    buildLut1dU8((value) => math.pow(value, exponent).toDouble());
