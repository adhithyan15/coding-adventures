/// Field arithmetic, point operations, and scalar arithmetic for
/// Ed25519, over exact [BigInt] integers.
///
/// The reference represents GF(2^255-19) elements as five 64-bit limbs
/// in radix 2^51 with hand-rolled carry chains. Dart has exact
/// arbitrary-precision integers instead, so field elements are plain
/// [BigInt] values reduced mod p after every operation, and scalars
/// are [BigInt] values reduced mod L. The observable behavior —
/// encodings, vectors, verification verdicts — is identical; the
/// addition chains, limb packing, and schoolbook division simply have
/// no reason to exist here.
library;

import 'constants.dart';

/// A field element in GF(2^255 - 19), always fully reduced.
class Fe {
  /// The value in `0..p`.
  final BigInt value;

  /// Creates a reduced field element.
  Fe(BigInt v) : value = v % ed25519P;

  /// The additive identity.
  static final Fe zero = Fe(BigInt.zero);

  /// The multiplicative identity.
  static final Fe one = Fe(BigInt.one);

  /// Decodes from 32 little-endian bytes. Masks off bit 255.
  factory Fe.fromBytes(List<int> bytes) {
    var v = BigInt.zero;
    for (var i = 30; i >= 0; i--) {
      v = (v << 8) | BigInt.from(bytes[i] & 0xff);
    }
    // Byte 31 contributes only its low 7 bits (bit 255 masked off).
    v |= (BigInt.from(bytes[31] & 0x7f) << (8 * 31));
    return Fe(v);
  }

  /// Encodes as 32 little-endian bytes (fully reduced by construction).
  Uint8ListBytes toBytes() {
    final out = Uint8ListBytes(32);
    var v = value;
    for (var i = 0; i < 32; i++) {
      out[i] = (v & BigInt.from(0xff)).toInt();
      v >>= 8;
    }
    return out;
  }

  /// Field addition.
  Fe operator +(Fe other) => Fe(value + other.value);

  /// Field subtraction.
  Fe operator -(Fe other) => Fe(value - other.value);

  /// Field negation.
  Fe operator -() => Fe(-value);

  /// Field multiplication.
  Fe operator *(Fe other) => Fe(value * other.value);

  /// Field squaring.
  Fe square() => Fe(value * value);

  /// Modular inverse via Fermat: a^(p-2) mod p.
  Fe invert() => Fe(value.modPow(ed25519P - BigInt.two, ed25519P));

  /// Square root: a^((p+3)/8), then check/fix.
  Fe? sqrt() {
    final candidate =
        Fe(value.modPow((ed25519P + BigInt.from(3)) >> 3, ed25519P));
    if (candidate.square() == this) {
      return candidate;
    }
    if (candidate.square() == -this) {
      return candidate * Fe.fromBytes(ed25519SqrtM1Bytes);
    }
    return null;
  }

  @override
  bool operator ==(Object other) => other is Fe && value == other.value;

  @override
  int get hashCode => value.hashCode;
}

/// Minimal fixed-length byte list for internal encodings.
///
/// (The public API uses [Uint8List]; this tiny class keeps the private
/// math free of `dart:typed_data` conversions in hot loops.)
class Uint8ListBytes {
  final List<int> _bytes;

  Uint8ListBytes(int length) : _bytes = List<int>.filled(length, 0);

  int operator [](int i) => _bytes[i];
  void operator []=(int i, int v) => _bytes[i] = v;
  List<int> toList() => List<int>.of(_bytes);
}

/// A point on the Ed25519 curve in extended twisted Edwards
/// coordinates.
class ExtPoint {
  final Fe x;
  final Fe y;
  final Fe z;
  final Fe t;

  const ExtPoint(this.x, this.y, this.z, this.t);

  /// The identity element: affine (0, 1).
  factory ExtPoint.identity() => ExtPoint(Fe.zero, Fe.one, Fe.one, Fe.zero);

  /// The base point B.
  factory ExtPoint.base() {
    final bx = Fe.fromBytes(ed25519BxBytes);
    final by = Fe.fromBytes(ed25519ByBytes);
    return ExtPoint(bx, by, Fe.one, bx * by);
  }

  /// Point addition (twisted Edwards, a = -1).
  ExtPoint add(ExtPoint other) {
    final dFe = Fe.fromBytes(ed25519DBytes);
    final a = x * other.x;
    final b = y * other.y;
    final c = t * dFe * other.t;
    final dd = z * other.z;
    final e = ((x + y) * (other.x + other.y)) - a - b;
    final f = dd - c;
    final g = dd + c;
    final h = b + a;
    return ExtPoint(e * f, g * h, f * g, e * h);
  }

  /// Point doubling.
  ExtPoint double() {
    final a = x.square();
    final b = y.square();
    final c = z.square() + z.square();
    final dd = -a;
    final e = ((x + y).square()) - a - b;
    final g = dd + b;
    final f = g - c;
    final h = dd - b;
    return ExtPoint(e * f, g * h, f * g, e * h);
  }

  /// Scalar multiplication: double-and-add from MSB to LSB.
  ExtPoint scalarMul(List<int> scalar) {
    var result = ExtPoint.identity();
    var started = false;

    // Iterate bytes from MSB (byte 31) to LSB (byte 0).
    for (var i = scalar.length - 1; i >= 0; i--) {
      final byte = scalar[i];
      for (var bit = 7; bit >= 0; bit--) {
        if (started) {
          result = result.double();
        }
        if ((byte >> bit) & 1 == 1) {
          if (!started) {
            result = this;
            started = true;
          } else {
            result = result.add(this);
          }
        }
      }
    }

    if (!started) {
      return ExtPoint.identity();
    }
    return result;
  }

  /// Encodes the point as 32 bytes per RFC 8032.
  Uint8ListBytes encode() {
    final zInv = z.invert();
    final xAff = x * zInv;
    final yAff = y * zInv;
    final encoded = yAff.toBytes();
    final xBytes = xAff.toBytes();
    encoded[31] |= (xBytes[0] & 1) << 7;
    return encoded;
  }

  /// Decodes a 32-byte compressed point. Returns null when invalid.
  static ExtPoint? decode(List<int> data) {
    final sign = (data[31] >> 7) & 1;
    final yBytes = List<int>.of(data);
    yBytes[31] &= 0x7f;
    final y = Fe.fromBytes(yBytes);

    // Check y < p: encode the reduced value and compare.
    final reduced = y.toBytes().toList();
    for (var i = 0; i < 32; i++) {
      if (reduced[i] != yBytes[i]) return null;
    }

    final dFe = Fe.fromBytes(ed25519DBytes);
    final ySq = y.square();
    final num = ySq - Fe.one;
    final den = dFe * ySq + Fe.one;
    final xSq = num * den.invert();

    final xSqBytes = xSq.toBytes().toList();
    if (xSqBytes.every((b) => b == 0)) {
      if (sign == 1) return null;
      return ExtPoint(Fe.zero, y, Fe.one, Fe.zero);
    }

    final x = xSq.sqrt();
    if (x == null) return null;

    final xRaw = x.toBytes().toList();
    final xFinal = (xRaw[0] & 1) != sign ? -x : x;
    return ExtPoint(xFinal, y, Fe.one, xFinal * y);
  }
}

/// Affine point equality without inversion.
bool pointEq(ExtPoint left, ExtPoint right) {
  final lxRz = left.x * right.z;
  final rxLz = right.x * left.z;
  final lyRz = left.y * right.z;
  final ryLz = right.y * left.z;
  return lxRz == rxLz && lyRz == ryLz;
}

/// Reduces an arbitrary-size little-endian integer mod L, returning 32
/// little-endian bytes.
Uint8ListBytes scReduce(List<int> bytes) {
  var v = BigInt.zero;
  for (var i = bytes.length - 1; i >= 0; i--) {
    v = (v << 8) | BigInt.from(bytes[i] & 0xff);
  }
  return _le32(v % ed25519L);
}

/// Computes (r + k*a) mod L over 32-byte scalars.
Uint8ListBytes scMulAdd(List<int> r, List<int> k, List<int> a) {
  BigInt load(List<int> b) {
    var v = BigInt.zero;
    for (var i = b.length - 1; i >= 0; i--) {
      v = (v << 8) | BigInt.from(b[i] & 0xff);
    }
    return v;
  }

  return _le32((load(r) + load(k) * load(a)) % ed25519L);
}

/// Checks whether a 32-byte LE scalar is canonical (< L).
bool scIsCanonical(List<int> s) {
  for (var i = 31; i >= 0; i--) {
    if (s[i] < ed25519LBytes[i]) return true;
    if (s[i] > ed25519LBytes[i]) return false;
  }
  return false; // s == L
}

Uint8ListBytes _le32(BigInt v) {
  final out = Uint8ListBytes(32);
  var rest = v;
  for (var i = 0; i < 32; i++) {
    out[i] = (rest & BigInt.from(0xff)).toInt();
    rest >>= 8;
  }
  return out;
}

/// Decodes a prime-order, non-identity point. Returns null for
/// non-canonical encodings, low-order points, and torsion-mixed points.
ExtPoint? decodePrimeOrderNonIdentity(List<int> encoded) {
  final point = ExtPoint.decode(encoded);
  if (point == null) return null;
  final identity = ExtPoint.identity();
  final timesL = point.scalarMul(ed25519LBytes);
  if (pointEq(point, identity) || !pointEq(timesL, identity)) {
    return null;
  }
  return point;
}

/// Decodes a point that is not of small order (kills cofactor-8
/// torsion with three doublings instead of a full scalar multiply).
ExtPoint? decodeNonSmallOrder(List<int> encoded) {
  final point = ExtPoint.decode(encoded);
  if (point == null) return null;
  final byCofactor = point.double().double().double();
  if (pointEq(byCofactor, ExtPoint.identity())) {
    return null;
  }
  return point;
}
