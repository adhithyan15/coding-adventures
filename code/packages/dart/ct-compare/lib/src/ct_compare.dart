// Constant-time comparison and selection (CT01), Dart lane.
//
// Why this exists
// ---------------
// A secret comparison that stops at the first differing byte tells an attacker
// how long the matching prefix was. Controlling one operand (a MAC tag, a
// session token) they can recover the other a byte at a time by timing guesses.
// The defence is to read EVERY byte whatever the data is, fold the differences
// into one accumulator, and look at the accumulator once at the very end:
//
//     a   = 41 42 43 44          a   = 41 42 43 44
//     b   = 41 42 43 44          b   = 41 FF 43 44
//     xor = 00 00 00 00          xor = 00 BD 00 00   (all four bytes were read)
//     acc = 0x00 -> equal        acc = 0xBD          -> different
//
// What this library promises -- and what it does not
// --------------------------------------------------
// The SOURCE has no content-dependent early return, branch, index or loop
// bound; loops run to the public length. It does NOT promise wall-clock
// constant time, and Dart cannot:
//
//  * On the Dart VM the JIT may recompile hot loops with different shapes, and
//    the AOT compiler may rewrite them; nothing lets the source pin machine
//    code. When compiled to JavaScript, the JS engine's JIT sits underneath as
//    well.
//  * The garbage collector can run at any allocation and moves objects, so
//    secret bytes may be copied and left behind in freed memory.
//  * Typed-data bounds checks branch on the public index, never on data.
//  * Cache and speculation behaviour of the host are outside the model.
//
// The selection bit
// -----------------
// Dart has no branch-free way to turn a bool into a number (`b ? 1 : 0` is a
// branch on the secret), so, like the Go, Ruby and Elixir lanes, the bit is an
// int that must be exactly 0 (pick right) or 1 (pick left), and every equality
// has a bit-returning variant:
//
//     ctSelectBytes(a, b, ctEqBit(x, y))
//
// converts no boolean.
//
// Portability: Dart ints are 64-bit on the VM but 53-bit doubles with 32-bit
// bitwise operators when compiled to JavaScript. This file therefore uses only
// 8-bit and 32-bit values in its bit tricks, so both targets agree.

import 'dart:typed_data';

/// A failed precondition of [ctSelectBytes] or [ctEqU64].
///
/// [id] is `CT_SELECT_LENGTH_MISMATCH` (CT01's semantic error),
/// `CT_INVALID_CHOICE` or `CT_INVALID_U64`. All are decided from public
/// information, before any secret content is read.
class CtCompareError implements Exception {
  CtCompareError(this.id, this.message);

  final String id;
  final String message;

  @override
  String toString() => 'CtCompareError($id): $message';
}

// OR together the XOR of every byte pair. Both lists have the same length; the
// loop bound is that public length.
int _difference(Uint8List left, Uint8List right) {
  var acc = 0;
  for (var i = 0; i < left.length; i++) {
    acc |= left[i] ^ right[i];
  }
  return acc;
}

/// `1` when the byte strings are equal, else `0`. A length mismatch is `0` and
/// reads neither input's contents; lengths are public.
///
/// For an accumulator `acc` in 0..255, `acc - 1` is -1 exactly when `acc == 0`
/// and otherwise 0..254. A logical shift right by 8 leaves a non-zero value
/// for -1 (all ones) and zero for 0..254, and `& 1` keeps the one bit that
/// distinguishes them: 1 for "equal", 0 for "different". No comparison, no
/// conditional.
int ctEqBit(Uint8List left, Uint8List right) {
  if (left.length != right.length) {
    return 0;
  }
  return ((_difference(left, right) - 1) >>> 8) & 1;
}

/// Fixed-width companion of [ctEqBit]; Dart has no fixed-size byte type, so
/// it is the same operation, kept so call sites read the same in every lane.
int ctEqFixedBit(Uint8List left, Uint8List right) => ctEqBit(left, right);

/// Boolean equality without early exit. The boolean is the public result.
bool ctEq(Uint8List left, Uint8List right) => ctEqBit(left, right) == 1;

/// Boolean form of [ctEqFixedBit].
bool ctEqFixed(Uint8List left, Uint8List right) => ctEqBit(left, right) == 1;

/// A fresh copy of [left] when [choice] is 1 and of [right] when it is 0.
///
/// Throws [CtCompareError] `CT_SELECT_LENGTH_MISMATCH` for unequal lengths and
/// `CT_INVALID_CHOICE` for any other [choice]; both are decided before either
/// input's contents are read (the length check first).
///
/// With `mask = 0xFF` (choice 1) or `0x00` (choice 0):
///
///     out = right ^ ((left ^ right) & mask)
///
/// is `left` or `right` by the same instructions either way.
///
/// The validity test is `choice * choice != choice`: it holds exactly for 0 and
/// 1, is one computation giving the same answer for both, and behaves the same
/// on the VM (64-bit wraparound: n(n-1) divisible by 2^64 forces n to be 0 or
/// 1) and in JavaScript (where `& ~1` would silently truncate a value such as
/// 2^32 to 0).
Uint8List ctSelectBytes(Uint8List left, Uint8List right, int choice) {
  if (left.length != right.length) {
    throw CtCompareError(
      'CT_SELECT_LENGTH_MISMATCH',
      'ctSelectBytes requires equal-length byte strings',
    );
  }
  if (choice * choice != choice) {
    throw CtCompareError(
      'CT_INVALID_CHOICE',
      'ctSelectBytes requires choice to be the integer 0 or 1',
    );
  }
  final mask = (-choice) & 0xFF; // 0 -> 0x00, 1 -> 0xFF
  final out = Uint8List(left.length);
  for (var i = 0; i < left.length; i++) {
    final l = left[i];
    final r = right[i];
    out[i] = r ^ ((l ^ r) & mask);
  }
  return out;
}

final BigInt _u64Max = (BigInt.one << 64) - BigInt.one;
final BigInt _low32 = BigInt.from(0xFFFFFFFF);

/// `1` when the two unsigned 64-bit values are equal, else `0`.
///
/// The values are [BigInt]s so that all 64 bits are exact on every Dart
/// target. They must lie in `0..2^64-1` ([CtCompareError] `CT_INVALID_U64`
/// otherwise; range is public). [BigInt] arithmetic is not constant time, so
/// only the reduction below is: each value is split into two 32-bit halves and
/// the XOR of the halves is OR-folded into one 32-bit difference `d`.
/// `d | -d` has bit 31 set exactly when `d != 0` (for non-zero `d` either `d`
/// or its two's complement has it set), so shifting that bit down and flipping
/// it gives the equality bit.
int ctEqU64Bit(BigInt left, BigInt right) {
  _checkU64(left, 'left');
  _checkU64(right, 'right');
  final hi = (left >> 32).toInt() ^ (right >> 32).toInt();
  final lo = (left & _low32).toInt() ^ (right & _low32).toInt();
  final d = hi | lo; // 0 .. 2^32-1, zero exactly when the values are equal
  final folded = ((d | -d) & 0xFFFFFFFF) >> 31;
  return folded ^ 1;
}

/// Boolean form of [ctEqU64Bit].
bool ctEqU64(BigInt left, BigInt right) => ctEqU64Bit(left, right) == 1;

void _checkU64(BigInt value, String name) {
  if (value.isNegative || value > _u64Max) {
    throw CtCompareError(
      'CT_INVALID_U64',
      '$name must be an unsigned 64-bit integer',
    );
  }
}
