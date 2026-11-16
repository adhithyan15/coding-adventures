/// Trigonometry from first principles (TRIG-REF-1).
///
/// This library computes sine, cosine and their friends from **addition,
/// subtraction, multiplication and comparison alone**.  It never calls
/// `dart:math`'s `sin` or `cos`, and `sin`, `cos`, `radians` and `degrees` do
/// not even divide.  Every number is a binary64 `double` and every operation
/// rounds once, so the answers are *bit for bit* the same as the other
/// language ports of this package.  The algorithm is specified in
/// `code/specs/PHY00a-trig-convergence.md` and checked against the shared
/// vectors in `code/specs/fixtures/trig-reference-v1`.
///
/// **Scope of the claim: the Dart VM** (JIT and AOT).  Compiled to JavaScript
/// (dart2js) numbers are JavaScript numbers and nothing here has been run
/// there, so no bit-exactness is claimed on the web.
///
/// ## The big idea
///
/// A sine wave repeats, so knowing `sin` on the small interval [-pi/4, pi/4]
/// and *which quarter-turn* an angle sits in is enough to know it everywhere:
///
///     x = k * (pi/2) + r        with  |r| <= pi/4  and  k a whole number
///
/// * `sin x` is one of `sin r`, `cos r`, `-sin r`, `-cos r`, chosen by `k mod 4`.
/// * `cos x` is one of `cos r`, `-sin r`, `-cos r`, `sin r`.
///
/// So the work is (1) finding `k` and `r` accurately and (2) evaluating
/// `sin r` and `cos r` for small `r` by their Maclaurin series.
///
/// ## Step 1: range reduction (Cody-Waite)
///
/// `k` is the whole number nearest `x / (pi/2)`.  We find it with an
/// add-and-subtract trick: `magic = 1.5 * 2^52` has its units digit in the last
/// place of the significand, so `(t + magic) - magic` is `t` rounded to the
/// nearest integer using only `+` and `-` (no `floor`, no `round`, no cast, so
/// nothing special happens at NaN or infinity).
///
/// Then `r = x - k*(pi/2)`.  The catch: `pi/2` is irrational, so a 53-bit
/// `pi/2` times a big `k` loses the very digits we need.  The fix is to store
/// `pi/2` as a sum of five short pieces, each at most 24 bits, so that
/// `k * piece` is **exact** (24 bits + 29 bits of `k` = 53 bits):
///
///     x ------------------------------------------------------ the angle
///     |-- k*pio2_1 --|                                          24 bits of k*pi/2
///                    |-- k*pio2_2 --|                           next 24
///                                   |-- k*pio2_3 --|           next 24
///                                                  |- k*pio2_4 -|  next 24
///                                                               |k*pio2_5| rounds
///
/// Each subtraction strips one more slice of `k*pi/2` off `x`.
///
/// ## Step 2: the series
///
/// On |r| <= pi/4 both functions are short polynomials in `z = r*r`:
///
///     sin r = r + r*z*(S1 + S2*z + ... + S9*z^8)       S[n] = (-1)^n / (2n+1)!
///     cos r = 1 - z/2 + z*z*(C2 + C3*z + ... + C9*z^7) C[n] = (-1)^n / (2n)!
///
/// The coefficients are summed by **Horner's rule**, innermost first: the
/// smallest terms are added first, which is the accurate order, and each step
/// is one multiply and one add.
///
/// ## Accuracy
///
/// `sin` and `cos` are within 3 ulp of the exact value on the whole domain
/// (the vectors gate this; the worst found is 2.38 ulp).  `radians` and
/// `degrees` are within 1 ulp.  `tan`, `sqrt`, `atan` and `atan2` are pinned
/// conventions or legacy algorithms that make **no accuracy claim**.
///
/// ## The domain
///
/// `sin`, `cos` and `tan` are defined for `|x| <= 2^29` (about 5.4e8 radians).
/// Beyond that, and for NaN and the infinities, they return NaN: the quotient
/// `k` would no longer fit the exact products above, and a finite answer there
/// would be a wrong answer that looks like a right one.
///
/// ## No fused multiply-add
///
/// Every product and the add or subtract that uses it are separate statements.
/// A compiler that fused `r - k*p` into one fused multiply-add would round once
/// instead of twice and change the last bit.  The Dart VM (JIT and AOT) does
/// not contract; the vectors enforce the visible sites and the test suite is
/// also run AOT-compiled.
library;

import 'constants.dart';

/// Pi to full IEEE 754 double precision.
const double pi = 3.141592653589793;

/// One complete rotation in radians.
const double twoPi = 2.0 * pi;

/// A quarter rotation in radians.
const double halfPi = pi / 2.0;

/// The five pieces of pi/2, most significant first.
final List<double> _pio2 = [pio2_1, pio2_2, pio2_3, pio2_4, pio2_5];

/// Sine coefficients; index 0 is unused.
final List<double> _s = [0.0, s1, s2, s3, s4, s5, s6, s7, s8, s9];

/// Cosine coefficients; indices 0 and 1 are unused (the -1/2 term is `0.5 * z`).
final List<double> _c = [0.0, 0.0, c2, c3, c4, c5, c6, c7, c8, c9];

// ---------------------------------------------------------------------------
// Range reduction
// ---------------------------------------------------------------------------

/// Returns `x = k*(pi/2) + r` as the pair `(k, r)` with `|r| <= pi/4` (about).
///
/// Precondition: `|x| <= 2^29` (the callers' domain guard has run), so `k` is a
/// finite whole number below 2^29 in magnitude.
(double, double) _reduce(double x) {
  final t = x * invPio2; // x in quarter-turns, one rounding
  final k = (t + magic) - magic; // nearest whole number, ties to even
  var r = x;
  for (final piece in _pio2) {
    final m = k * piece; // exact for the first four pieces
    r = r - m;
  }
  return (k, r);
}

// ---------------------------------------------------------------------------
// The series kernels, Horner's rule on stored coefficients
// ---------------------------------------------------------------------------

/// sin(r) for |r| <= pi/4:  r + r*z*(S1 + S2*z + ... + S9*z^8).
double _sinKernel(double r) {
  final z = r * r;
  var p = _s[9];
  for (var n = 8; n >= 1; n--) {
    p = p * z;
    p = p + _s[n];
  }
  var w = r * z;
  w = w * p;
  return r + w;
}

/// cos(r) for |r| <= pi/4:  (1 - z/2) + z*z*(C2 + C3*z + ... + C9*z^7).
double _cosKernel(double r) {
  final z = r * r;
  var q = _c[9];
  for (var n = 8; n >= 2; n--) {
    q = q * z;
    q = q + _c[n];
  }
  var u = 0.5 * z; // exact
  u = 1.0 - u;
  var v = z * z;
  v = v * q;
  return u + v;
}

// ---------------------------------------------------------------------------
// Sine and cosine
// ---------------------------------------------------------------------------

/// Sine of [x] radians, for `|x| <= 2^29`; NaN outside that domain.
///
///     input                 result
///     NaN, +-inf            NaN
///     |x| > 2^29            NaN (outside the domain)
///     +0 / -0               +0 / -0  (sin(-0) = -0)
///     0 < |x| < 2^-27       x itself (correctly rounded)
///     otherwise             reduce, then pick a kernel
///
/// The domain guard is a *negated* `<=`.  A NaN fails every comparison, so
/// `!(x.abs() <= limit)` is true for NaN, both infinities and every large
/// finite value in one test.  The tempting `x.abs() > limit` lets a NaN through.
double sin(double x) {
  if (!(x.abs() <= limit)) {
    return double.nan;
  }
  if (x.abs() < tiny) {
    return x; // also keeps -0.0 and subnormals exact
  }
  final (k, r) = _reduce(x);
  // k is a whole number below 2^29, so toInt() is exact; "& 3" is k mod 4
  // (floor-modulus for negatives: -1 & 3 == 3).
  switch (k.toInt() & 3) {
    case 0:
      return _sinKernel(r);
    case 1:
      return _cosKernel(r);
    case 2:
      return -_sinKernel(r);
    default:
      return -_cosKernel(r);
  }
}

/// Cosine of [x] radians, for `|x| <= 2^29`; NaN outside that domain.
///
/// `cos(+-0) = 1`, and `cos(x) = 1` for `|x| < 2^-27` (the first correction,
/// `x^2/2`, is below half an ulp of the double just under 1).  The quarter-turn
/// table is `cos r, -sin r, -cos r, sin r` for `k mod 4 = 0, 1, 2, 3`.
double cos(double x) {
  if (!(x.abs() <= limit)) {
    return double.nan;
  }
  if (x.abs() < tiny) {
    return 1.0;
  }
  final (k, r) = _reduce(x);
  switch (k.toInt() & 3) {
    case 0:
      return _cosKernel(r);
    case 1:
      return -_sinKernel(r);
    case 2:
      return -_cosKernel(r);
    default:
      return _sinKernel(r);
  }
}

/// Converts degrees to radians: ONE multiply by `pi/180`.
///
/// The constant is pre-divided and stored as a bit pattern, so the conversion
/// rounds once (`degrees * pi / 180` rounds twice and can overflow its
/// intermediate).  NaN propagates, infinities map to infinities and signed zero
/// keeps its sign.
double radians(double degrees) => degrees * radPerDeg;

/// Converts radians to degrees: ONE multiply by `180/pi`.
double degrees(double radians) => radians * degPerRad;

// ---------------------------------------------------------------------------
// Part II: pinned legacy algorithms (no accuracy claim)
// ---------------------------------------------------------------------------

/// Computes a real square root with Newton's method.
///
/// Throws [ArgumentError] when [value] is negative.
double sqrt(double value) {
  if (value < 0.0) {
    throw ArgumentError.value(value, 'value', 'must be non-negative');
  }
  return _sqrtNonnegative(value);
}

/// Computes tangent as sine over cosine, with a pinned pole sentinel.
///
/// Wherever `cos x = 0` the tangent is undefined.  When `|cos x| < 1e-15` this
/// returns +-1e308 (signed like `sin`) instead of an infinity.  That is *a
/// convention, not an approximation*: the true tangent of the double nearest
/// pi/2 is about 1.6e16.  PHY00 documents it and the shared vectors require it.
/// **No accuracy claim is made for `tan`.**
///
/// `tan(+-0) = +-0`; NaN, infinities and `|x| > 2^29` give NaN (both `sin` and
/// `cos` are NaN, the sentinel test is false for NaN, and NaN / NaN is NaN).
double tan(double x) {
  final sine = sin(x);
  final cosine = cos(x);
  if (cosine.abs() < tanPole) {
    return sine > 0.0 ? tanBig : -tanBig;
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

/// Computes the four-quadrant arctangent of ([y], [x]), in `(-pi, pi]`.
///
/// These are *pinned conventions*, not IEEE 754's: `atan2(-0, x < 0)` is `+pi`
/// and `atan2(+-inf, +-inf)` is NaN (because `y/x` is NaN).  A NaN in either
/// argument gives NaN: a NaN fails every comparison below and would otherwise
/// fall through to the last branch, a finite answer to an undefined question.
double atan2(double y, double x) {
  if (x.isNaN || y.isNaN) {
    return double.nan;
  }
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
    if ((next - guess).abs() < sqrtRel * guess + sqrtTiny) {
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

  // Each term is the previous one times (-t^2) * (2n-1) / (2n+1).  The three
  // operations run strictly left to right, one rounding each (PHY00a 3.9).  The
  // quotient-first form `term *= -t^2 * (2n-1) / (2n+1)` multiplies by a
  // pre-rounded ratio instead and differs in the last bit on about 0.24% of
  // inputs.
  for (var n = 1; n <= 30; n++) {
    final index = n.toDouble();
    term = term * -squared;
    term = term * (2.0 * index - 1.0);
    term = term / (2.0 * index + 1.0);
    total += term;
    if (term.abs() < atanTerm) {
      break;
    }
  }
  return 2.0 * total;
}
