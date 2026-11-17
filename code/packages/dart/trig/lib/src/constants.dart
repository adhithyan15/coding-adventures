/// The TRIG-REF-1 constant table (PHY00a 3.2), built from bit patterns.
///
/// A constant is a bit pattern; the decimal is only a convenience.  Dart has no
/// hexadecimal float literal, so each constant is two 32-bit halves of its
/// IEEE 754 binary64 encoding, assembled through a [ByteData].  No decimal is
/// ever parsed, so no compiler or platform can round one wrongly.
///
/// Two halves, not one 64-bit integer: a 64-bit integer literal and
/// `ByteData.setUint64` are not usable when Dart is compiled to JavaScript, and
/// 32-bit halves work everywhere.  (This package claims the Dart VM only; the
/// halves cost nothing and keep the door open.)
///
/// The test suite compares every entry of [referenceConstants] against the
/// manifest's table (PHY00a 6.5 e), so a mistyped digit cannot survive.
library;

import 'dart:typed_data';

import 'trig.dart' show halfPi, pi, twoPi;

final ByteData _scratch = ByteData(8);

/// The binary64 value whose IEEE 754 encoding is `hi:lo` (high word first).
double _fromBits(int hi, int lo) {
  _scratch.setUint32(0, hi);
  _scratch.setUint32(4, lo);
  return _scratch.getFloat64(0);
}

/// pi/2, first 24-bit piece (1.570796251296997).
final double pio2_1 = _fromBits(0x3ff921fb, 0x40000000);

/// pi/2, second piece (7.549789415861596e-08).
final double pio2_2 = _fromBits(0x3e74442d, 0x00000000);

/// pi/2, third piece (5.390302529957765e-15).
final double pio2_3 = _fromBits(0x3cf84698, 0x80000000);

/// pi/2, fourth piece (3.282003415807913e-22).
final double pio2_4 = _fromBits(0x3b78cc51, 0x60000000);

/// pi/2, the rounded remainder (1.2706558760139879e-29).
final double pio2_5 = _fromBits(0x39f01b83, 0x9a25204a);

/// 2/pi, the double nearest to it: x * invPio2 is x in quarter-turns.
final double invPio2 = _fromBits(0x3fe45f30, 0x6dc9c883);

/// 1.5 * 2^52: adding then subtracting it rounds to the nearest integer.
final double magic = _fromBits(0x43380000, 0x00000000);

/// 2^29, the edge of the domain of sin, cos and tan.
final double limit = _fromBits(0x41c00000, 0x00000000);

/// 2^-27, below which sin(x) == x and cos(x) == 1 exactly.
final double tiny = _fromBits(0x3e400000, 0x00000000);

/// Sine coefficient S1 = (-1)^1 / (2*1+1)!.
final double s1 = _fromBits(0xbfc55555, 0x55555555);

/// Sine coefficient S2 = (-1)^2 / (2*2+1)!.
final double s2 = _fromBits(0x3f811111, 0x11111111);

/// Sine coefficient S3 = (-1)^3 / (2*3+1)!.
final double s3 = _fromBits(0xbf2a01a0, 0x1a01a01a);

/// Sine coefficient S4 = (-1)^4 / (2*4+1)!.
final double s4 = _fromBits(0x3ec71de3, 0xa556c734);

/// Sine coefficient S5 = (-1)^5 / (2*5+1)!.
final double s5 = _fromBits(0xbe5ae645, 0x67f544e4);

/// Sine coefficient S6 = (-1)^6 / (2*6+1)!.
final double s6 = _fromBits(0x3de61246, 0x13a86d09);

/// Sine coefficient S7 = (-1)^7 / (2*7+1)!.
final double s7 = _fromBits(0xbd6ae7f3, 0xe733b81f);

/// Sine coefficient S8 = (-1)^8 / (2*8+1)!.
final double s8 = _fromBits(0x3ce952c7, 0x7030ad4a);

/// Sine coefficient S9 = (-1)^9 / (2*9+1)!.
final double s9 = _fromBits(0xbc62f49b, 0x46814157);

/// Cosine coefficient C2 = (-1)^2 / (2*2)!.
final double c2 = _fromBits(0x3fa55555, 0x55555555);

/// Cosine coefficient C3 = (-1)^3 / (2*3)!.
final double c3 = _fromBits(0xbf56c16c, 0x16c16c17);

/// Cosine coefficient C4 = (-1)^4 / (2*4)!.
final double c4 = _fromBits(0x3efa01a0, 0x1a01a01a);

/// Cosine coefficient C5 = (-1)^5 / (2*5)!.
final double c5 = _fromBits(0xbe927e4f, 0xb7789f5c);

/// Cosine coefficient C6 = (-1)^6 / (2*6)!.
final double c6 = _fromBits(0x3e21eed8, 0xeff8d898);

/// Cosine coefficient C7 = (-1)^7 / (2*7)!.
final double c7 = _fromBits(0xbda93974, 0xa8c07c9d);

/// Cosine coefficient C8 = (-1)^8 / (2*8)!.
final double c8 = _fromBits(0x3d2ae7f3, 0xe733b81f);

/// Cosine coefficient C9 = (-1)^9 / (2*9)!.
final double c9 = _fromBits(0xbca68278, 0x63b97d97);

/// pi/180, pre-divided so a conversion is one multiply.
final double radPerDeg = _fromBits(0x3f91df46, 0xa2529d39);

/// 180/pi, pre-divided so a conversion is one multiply.
final double degPerRad = _fromBits(0x404ca5dc, 0x1a63c1f8);

/// Part II (pinned legacy): the `atan` series stops once a term is below this.
const double atanTerm = 1e-17;

/// Part II: relative stopping tolerance of the `sqrt` Newton iteration.
const double sqrtRel = 1e-15;

/// Part II: absolute stopping tolerance of the `sqrt` Newton iteration.
const double sqrtTiny = 1e-300;

/// Part II: the magnitude of the signed `tan` pole sentinel.
const double tanBig = 1e308;

/// Part II: `tan` returns the sentinel when `|cos x|` is below this.
const double tanPole = 1e-15;

/// Every constant above by its manifest name, for the bit-for-bit test.
final Map<String, double> referenceConstants = {
  'PI': pi,
  'TWO_PI': twoPi,
  'HALF_PI': halfPi,
  'PIO2_1': pio2_1,
  'PIO2_2': pio2_2,
  'PIO2_3': pio2_3,
  'PIO2_4': pio2_4,
  'PIO2_5': pio2_5,
  'INV_PIO2': invPio2,
  'MAGIC': magic,
  'LIMIT': limit,
  'TINY': tiny,
  'S1': s1,
  'S2': s2,
  'S3': s3,
  'S4': s4,
  'S5': s5,
  'S6': s6,
  'S7': s7,
  'S8': s8,
  'S9': s9,
  'C2': c2,
  'C3': c3,
  'C4': c4,
  'C5': c5,
  'C6': c6,
  'C7': c7,
  'C8': c8,
  'C9': c9,
  'RAD_PER_DEG': radPerDeg,
  'DEG_PER_RAD': degPerRad,
  'ATAN_TERM': atanTerm,
  'SQRT_REL': sqrtRel,
  'SQRT_TINY': sqrtTiny,
  'TAN_BIG': tanBig,
  'TAN_POLE': tanPole,
};
