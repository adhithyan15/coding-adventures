/// Adder circuits built from logic gates: half adder, full adder, and
/// N-bit ripple-carry addition.
///
/// A bit is an `int` holding 0 or 1. Bit-domain rejection is inherited
/// from the logic-gates lane (its gates throw [ArgumentError] on non-bits,
/// always on); shape rejection (equal non-empty operand lengths) is this
/// module's own [ArgumentError]. Nothing is masked and nothing passes
/// through, exactly as in the reference.
library;

import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart'
    show AND, OR, XOR;

/// Half adder: adds two single bits, returning `(sum, carry)`.
/// `sum = a XOR b`, `carry = a AND b`.
(int, int) halfAdder(int a, int b) => (XOR(a, b), AND(a, b));

/// Full adder: adds two bits plus a carry-in, returning `(sum, carryOut)`.
///
/// Two half adders plus an OR gate: `carryOut = OR(partialCarry, carry2)`.
(int, int) fullAdder(int a, int b, int carryIn) {
  final (partialSum, partialCarry) = halfAdder(a, b);
  final (sum, carry2) = halfAdder(partialSum, carryIn);
  return (sum, OR(partialCarry, carry2));
}

/// Result of a ripple-carry addition.
class RippleCarryResult {
  /// Sum bits, LSB first (index 0 is least significant).
  final List<int> sum;

  /// Carry out of the most significant bit position.
  final int carryOut;

  /// Signed overflow: adding two positives produced a negative, or two
  /// negatives produced a positive. Meaningful for two's complement.
  final bool overflow;

  /// Creates a result.
  const RippleCarryResult({
    required this.sum,
    required this.carryOut,
    required this.overflow,
  });

  @override
  bool operator ==(Object other) =>
      other is RippleCarryResult &&
      _bitsEqual(sum, other.sum) &&
      carryOut == other.carryOut &&
      overflow == other.overflow;

  @override
  int get hashCode => Object.hash(Object.hashAll(sum), carryOut, overflow);

  @override
  String toString() =>
      'RippleCarryResult(sum: $sum, carryOut: $carryOut, overflow: $overflow)';
}

bool _bitsEqual(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

void _checkShapes(List<int> a, List<int> b) {
  if (a.length != b.length) {
    throw ArgumentError(
        'a and b must have the same length, got ${a.length} and ${b.length}');
  }
  if (a.isEmpty) {
    throw ArgumentError('bit lists must not be empty');
  }
}

/// Ripple-carry adder: adds two N-bit numbers (LSB-first lists) through a
/// chain of full adders, carry rippling from LSB to MSB.
///
/// Both lists must share one non-empty length; the sum is exactly that
/// long (no width-driven amplification). Throws [ArgumentError] on shape
/// violations or non-bit elements.
RippleCarryResult rippleCarryAdder(List<int> a, List<int> b) =>
    rippleCarryAdderWithCarry(a, b, 0);

/// Ripple-carry adder with an explicit carry-in (used for subtraction).
///
/// Computes `(a + b + carryIn) mod 2^n` with the carry out of bit `n - 1`
/// and the signed-overflow flag of the same sum. The carry-in rides the
/// first full adder, so a non-bit carry-in is rejected by the gates.
RippleCarryResult rippleCarryAdderWithCarry(
    List<int> a, List<int> b, int carryIn) {
  _checkShapes(a, b);
  final sum = <int>[];
  var carry = carryIn;
  for (var i = 0; i < a.length; i++) {
    final (s, c) = fullAdder(a[i], b[i], carry);
    sum.add(s);
    carry = c;
  }
  // Signed overflow: the sign bit disagrees with two agreeing operand
  // signs (equivalently, carry into the sign bit != carry out).
  final n = a.length;
  final overflow = a[n - 1] == b[n - 1] && sum[n - 1] != a[n - 1];
  return RippleCarryResult(sum: sum, carryOut: carry, overflow: overflow);
}
