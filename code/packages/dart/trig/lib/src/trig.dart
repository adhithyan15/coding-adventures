/// Pi to full IEEE 754 double precision.
const double pi = 3.141592653589793;

/// One complete rotation in radians.
const double twoPi = 2.0 * pi;

/// A quarter rotation in radians.
const double halfPi = pi / 2.0;

/// Computes sine with a 20-term Maclaurin series after range reduction.
double sin(double angle) {
  final reduced = _rangeReduce(angle);
  var term = reduced;
  var total = reduced;

  for (var n = 1; n < 20; n++) {
    final index = n.toDouble();
    final denominator = (2.0 * index) * (2.0 * index + 1.0);
    term *= -(reduced * reduced) / denominator;
    total += term;
  }
  return total;
}

/// Computes cosine with a 20-term Maclaurin series after range reduction.
double cos(double angle) {
  final reduced = _rangeReduce(angle);
  var term = 1.0;
  var total = 1.0;

  for (var n = 1; n < 20; n++) {
    final index = n.toDouble();
    final denominator = (2.0 * index - 1.0) * (2.0 * index);
    term *= -(reduced * reduced) / denominator;
    total += term;
  }
  return total;
}

/// Converts degrees to radians.
double radians(double degrees) => degrees * pi / 180.0;

/// Converts radians to degrees.
double degrees(double radians) => radians * 180.0 / pi;

/// Computes a real square root with Newton's method.
///
/// Throws [ArgumentError] when [value] is negative.
double sqrt(double value) {
  if (value < 0.0) {
    throw ArgumentError.value(value, 'value', 'must be non-negative');
  }
  return _sqrtNonnegative(value);
}

/// Computes tangent from this package's first-principles sine and cosine.
///
/// A signed, finite sentinel is returned within `1e-15` of a pole.
double tan(double angle) {
  final sine = sin(angle);
  final cosine = cos(angle);
  if (cosine.abs() < 1e-15) {
    return sine > 0.0 ? 1e308 : -1e308;
  }
  return sine / cosine;
}

/// Computes arctangent in the range `(-pi / 2, pi / 2)`.
double atan(double value) {
  // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
  if (value.abs() <= 7.450580596923828e-9) {
    return value;
  }
  if (value > 1.0) {
    return halfPi - _atanCore(1.0 / value);
  }
  if (value < -1.0) {
    return -halfPi - _atanCore(1.0 / value);
  }
  return _atanCore(value);
}

/// Computes the four-quadrant arctangent of ([y], [x]).
double atan2(double y, double x) {
  if (x > 0.0) {
    return atan(y / x);
  }
  if (x < 0.0 && y >= 0.0) {
    return atan(y / x) + pi;
  }
  if (x < 0.0 && y < 0.0) {
    return atan(y / x) - pi;
  }
  if (y > 0.0) {
    return halfPi;
  }
  if (y < 0.0) {
    return -halfPi;
  }
  return 0.0;
}

double _rangeReduce(double angle) {
  final wrapped = angle % twoPi;
  return wrapped > pi ? wrapped - twoPi : wrapped;
}

double _sqrtNonnegative(double value) {
  if (value == 0.0) {
    return value;
  }
  if (value.isInfinite) {
    return value;
  }

  // Normalize by powers of four so Newton iteration always starts on a
  // compact interval. This keeps the fixed iteration budget accurate for the
  // full finite-double exponent range, including subnormals, without using an
  // opaque standard-library square root.
  var scaled = value;
  var resultScale = 1.0;
  while (scaled < 0.25) {
    scaled *= 4.0;
    resultScale *= 0.5;
  }
  while (scaled >= 4.0) {
    scaled *= 0.25;
    resultScale *= 2.0;
  }

  var guess = scaled >= 1.0 ? scaled : 1.0;
  for (var iteration = 0; iteration < 60; iteration++) {
    final next = (guess + scaled / guess) / 2.0;
    if ((next - guess).abs() < 1e-15 * guess + 1e-300) {
      return next * resultScale;
    }
    guess = next;
  }
  return guess * resultScale;
}

double _atanCore(double value) {
  final reduced = value / (1.0 + _sqrtNonnegative(1.0 + value * value));
  final squared = reduced * reduced;
  var term = reduced;
  var total = reduced;

  for (var n = 1; n <= 30; n++) {
    final index = n.toDouble();
    term *= -squared * (2.0 * index - 1.0) / (2.0 * index + 1.0);
    total += term;
    if (term.abs() < 1e-17) {
      break;
    }
  }
  return 2.0 * total;
}
