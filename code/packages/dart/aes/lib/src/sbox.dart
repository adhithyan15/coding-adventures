/// AES S-box construction and the fixed-work substitution circuit.
///
/// The reference builds its tables with `gf256::Field::new(0x11B)`.
/// The in-lane gf256 package only implements the 0x11D polynomial,
/// so the 0x11B field arithmetic lives here: Russian-peasant
/// multiplication (the same algorithm the reference inlines for
/// InvMixColumns) plus inversion by exponentiation.
///
/// Public-in-source (not barrel-exported) so tests can pin the
/// arithmetic circuit against the tables, following the x25519
/// lane's field library.
library;

import 'dart:typed_data';

/// AES GF(2^8) reduction constant: x^8 + x^4 + x^3 + x + 1.
const int _reduction = 0x1b;

/// Russian-peasant multiplication in AES's GF(2^8).
int gfMul(int left, int right) {
  var product = 0;
  var a = left & 0xff;
  var b = right & 0xff;
  for (var i = 0; i < 8; i++) {
    if (b & 1 != 0) {
      product ^= a;
    }
    final hi = a & 0x80;
    a = (a << 1) & 0xff;
    if (hi != 0) {
      a ^= _reduction;
    }
    b >>= 1;
  }
  return product;
}

/// Multiplicative inverse in AES's GF(2^8): a^254, with 0 mapping to
/// 0 by the FIPS convention.
int gfInverse(int value) {
  value &= 0xff;
  if (value == 0) {
    return 0;
  }
  var result = 1;
  var base = value;
  var exp = 254;
  while (exp > 0) {
    if (exp & 1 != 0) {
      result = gfMul(result, base);
    }
    base = gfMul(base, base);
    exp >>= 1;
  }
  return result;
}

int _rotLeft8(int x, int n) {
  n &= 7;
  return ((x << n) | (x >> (8 - n))) & 0xff;
}

/// AES affine transform over GF(2): rotations by 1..4 XORed with the
/// input and the 0x63 constant (FIPS 197 §5.1.1 circulant form).
int affineTransform(int b) {
  b &= 0xff;
  return b ^
      _rotLeft8(b, 1) ^
      _rotLeft8(b, 2) ^
      _rotLeft8(b, 3) ^
      _rotLeft8(b, 4) ^
      0x63;
}

/// Builds the (SBOX, INV_SBOX) pair: GF inverse then affine per byte,
/// with the inverse table derived as INV[S[b]] = b.
(Uint8List, Uint8List) buildSboxes() {
  final sbox = Uint8List(256);
  for (var b = 0; b < 256; b++) {
    sbox[b] = affineTransform(b == 0 ? 0 : gfInverse(b));
  }
  final invSbox = Uint8List(256);
  for (var b = 0; b < 256; b++) {
    invSbox[sbox[b]] = b;
  }
  return (sbox, invSbox);
}

/// Branch-free GF(2^8) multiply: each multiplier bit becomes a
/// 0x00/0xff mask selecting the multiplicand arithmetically, and the
/// reduction uses the same mask construction for the high bit.
int gfMulFixed(int left, int right) {
  var product = 0;
  var a = left & 0xff;
  var b = right & 0xff;
  for (var i = 0; i < 8; i++) {
    final multiplierMask = -(b & 1) & 0xff;
    product ^= a & multiplierMask;
    final reductionMask = -(a >> 7) & 0xff;
    a = ((a << 1) ^ (_reduction & reductionMask)) & 0xff;
    b >>= 1;
  }
  return product;
}

/// Table-free S-box: x^254 by a fixed square-and-multiply chain
/// (mapping zero to zero, matching FIPS), then the affine circuit.
int sboxFixed(int value) {
  value &= 0xff;
  final x2 = gfMulFixed(value, value);
  final x4 = gfMulFixed(x2, x2);
  final x8 = gfMulFixed(x4, x4);
  final x16 = gfMulFixed(x8, x8);
  final x32 = gfMulFixed(x16, x16);
  final x64 = gfMulFixed(x32, x32);
  final x128 = gfMulFixed(x64, x64);

  final x6 = gfMulFixed(x2, x4);
  final x14 = gfMulFixed(x6, x8);
  final x30 = gfMulFixed(x14, x16);
  final x62 = gfMulFixed(x30, x32);
  final x126 = gfMulFixed(x62, x64);
  final inverse = gfMulFixed(x126, x128);
  return affineTransform(inverse);
}

/// Branch-free xtime: double in AES's field without testing the
/// high bit.
int xtimeFixed(int value) {
  value &= 0xff;
  return ((value << 1) ^ (_reduction & (-(value >> 7) & 0xff))) & 0xff;
}

/// Branching xtime for the compatibility path: shift left, XOR 0x1B
/// when bit 7 was set.
int xtime(int b) {
  b &= 0xff;
  final shifted = (b << 1) & 0xff;
  return b & 0x80 != 0 ? shifted ^ _reduction : shifted;
}
