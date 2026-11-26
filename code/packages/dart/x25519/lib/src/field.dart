/// Field arithmetic for X25519, over exact [BigInt] integers.
///
/// The reference represents GF(2^255-19) elements as five 64-bit
/// limbs in radix 2^51 with hand-rolled carry chains. Dart has exact
/// arbitrary-precision integers instead, so field elements are plain
/// [BigInt] values reduced mod p after every operation. The
/// observable behavior — encodings, ladder outputs, shared secrets —
/// is identical; the limb packing, carry chains, and addition chain
/// simply have no reason to exist here.
///
/// This library is public-in-source (like the ed25519 lane's curve
/// library) so tests can exercise the field operations the reference
/// tests pin; it is not exported from the package barrel.
library;

import 'dart:typed_data';

/// The field prime p = 2^255 - 19.
final BigInt x25519P = (BigInt.one << 255) - BigInt.from(19);

/// A field element in GF(2^255 - 19), always fully reduced.
class Fe {
  /// The value in `0..p`.
  final BigInt value;

  /// Creates a reduced field element.
  Fe(BigInt v) : value = v % x25519P;

  /// The additive identity.
  static final Fe zero = Fe(BigInt.zero);

  /// The multiplicative identity.
  static final Fe one = Fe(BigInt.one);

  /// Decodes from 32 little-endian bytes. Masks off bit 255 (the
  /// reference's limb extraction keeps bits 0..254 only) and reduces
  /// mod p, matching the reference's decode-then-reduce behavior on
  /// every input the ladder can present.
  factory Fe.fromBytes(List<int> bytes) {
    var v = BigInt.zero;
    for (var i = 30; i >= 0; i--) {
      v = (v << 8) | BigInt.from(bytes[i] & 0xff);
    }
    // Byte 31 contributes only its low 7 bits (bit 255 masked off).
    v |= (BigInt.from(bytes[31] & 0x7f) << (8 * 31));
    return Fe(v);
  }

  /// Encodes as 32 little-endian bytes (fully reduced by
  /// construction, so byte 31 is always below 0x80).
  Uint8List toBytes() {
    final out = Uint8List(32);
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

  /// Field multiplication.
  Fe operator *(Fe other) => Fe(value * other.value);

  /// Field squaring.
  Fe square() => Fe(value * value);

  /// Modular inverse via Fermat: a^(p-2) mod p.
  Fe invert() => Fe(value.modPow(x25519P - BigInt.two, x25519P));

  @override
  bool operator ==(Object other) => other is Fe && value == other.value;

  @override
  int get hashCode => value.hashCode;
}

/// Constant-time conditional swap: returns `(a, b)` when [swap] is 0
/// and `(b, a)` when it is 1, selected with a bitmask exactly like
/// the reference instead of a branch. Only the low bit of [swap] is
/// honored (the reference is undefined for other values; normalizing
/// keeps the ladder total). Like the ed25519 lane, this port makes no
/// full constant-time claim — [BigInt] operations are variable-time
/// regardless of the swap discipline.
(Fe, Fe) cswap(int swap, Fe a, Fe b) {
  // mask is all-ones below 2^255 when swap is odd, zero otherwise.
  final mask = (-BigInt.from(swap & 1)) & ((BigInt.one << 255) - BigInt.one);
  final diff = mask & (a.value ^ b.value);
  return (Fe(a.value ^ diff), Fe(b.value ^ diff));
}
