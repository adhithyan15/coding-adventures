/// X25519 elliptic-curve Diffie-Hellman on Curve25519 (RFC 7748).
///
/// Both parties combine a private scalar with the other's public
/// u-coordinate and arrive at the same 32-byte shared secret, while
/// an eavesdropper observing both public keys cannot derive it.
///
/// ## Field representation
///
/// The reference stores GF(p) elements as five 64-bit limbs in radix
/// 2^51. This port uses exact [BigInt] integers reduced mod p (see
/// `field.dart`, following the ed25519 lane): encodings, ladder
/// outputs, and shared secrets are identical, while limb packing,
/// carry chains, and the inversion addition chain have no reason to
/// exist here.
library;

import 'dart:typed_data';

import 'errors.dart';
import 'field.dart';

/// The standard base point: u = 9, little-endian.
const List<int> basePoint = [
  9,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
  0,
];

/// (A + 2) / 4 for Curve25519's A = 486662.
final Fe _a24 = Fe(BigInt.from(121666));

/// Clamps a 32-byte scalar (RFC 7748 §5): clears the low three bits
/// (cofactor clearing into the prime-order subgroup), clears bit 255,
/// and sets bit 254 (constant bit-length).
///
/// [x25519] clamps its scalar itself, so callers never have to call
/// this first. Throws [ArgumentError] unless [scalar] is 32 bytes.
Uint8List clampScalar(Uint8List scalar) {
  _checkLength(scalar, 32, 'scalar');
  final clamped = Uint8List.fromList(scalar);
  clamped[0] &= 248;
  clamped[31] &= 127;
  clamped[31] |= 64;
  return clamped;
}

Uint8List _ladder(Uint8List scalar, Uint8List uCoordinate) {
  final k = clampScalar(scalar);
  final uMasked = Uint8List.fromList(uCoordinate);
  uMasked[31] &= 0x7f;
  final u = Fe.fromBytes(uMasked);

  final x1 = u;
  var x2 = Fe.one;
  var z2 = Fe.zero;
  var x3 = u;
  var z3 = Fe.one;

  var swap = 0;
  for (var t = 254; t >= 0; t--) {
    final kT = (k[t ~/ 8] >> (t % 8)) & 1;
    swap ^= kT;
    (x2, x3) = cswap(swap, x2, x3);
    (z2, z3) = cswap(swap, z2, z3);
    swap = kT;

    final a = x2 + z2;
    final aa = a.square();
    final b = x2 - z2;
    final bb = b.square();
    final e = aa - bb;

    final c = x3 + z3;
    final d = x3 - z3;
    final da = d * a;
    final cb = c * b;

    x3 = (da + cb).square();
    z3 = x1 * (da - cb).square();
    x2 = aa * bb;
    z2 = e * (bb + _a24 * e);
  }

  (x2, x3) = cswap(swap, x2, x3);
  (z2, z3) = cswap(swap, z2, z3);

  return (x2 * z2.invert()).toBytes();
}

/// Computes X25519 scalar multiplication: the 32-byte u-coordinate
/// of `scalar` times the point at [uCoordinate].
///
/// Throws [ArgumentError] unless both inputs are 32 bytes, and
/// [X25519LowOrderPoint] when the result is all zeros (a low-order
/// point input, which must never silently produce a weak secret).
Uint8List x25519(Uint8List scalar, Uint8List uCoordinate) {
  _checkLength(scalar, 32, 'scalar');
  _checkLength(uCoordinate, 32, 'u-coordinate');
  final result = _ladder(scalar, uCoordinate);
  if (result.every((b) => b == 0)) {
    throw const X25519LowOrderPoint();
  }
  return result;
}

/// Scalar multiplication with the standard base point (u = 9): the
/// standard way to derive a public key from a private scalar.
///
/// Throws [ArgumentError] unless [scalar] is 32 bytes, and
/// [X25519LowOrderPoint] on an all-zeros result.
Uint8List x25519Base(Uint8List scalar) =>
    x25519(scalar, Uint8List.fromList(basePoint));

/// Derives the public key for [privateKey]: exactly [x25519Base],
/// named for API clarity. The input should be 32 bytes of
/// cryptographically secure random data.
Uint8List generateKeypair(Uint8List privateKey) => x25519Base(privateKey);

void _checkLength(List<int> bytes, int want, String what) {
  if (bytes.length != want) {
    throw ArgumentError('$what must be exactly $want bytes');
  }
}
