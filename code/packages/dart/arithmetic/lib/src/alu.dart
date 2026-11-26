/// Arithmetic Logic Unit: six operations over N-bit inputs plus flags.
///
/// [alu] dispatches on [AluOp]: [AluOp.add]/[AluOp.sub] run the
/// ripple-carry adder (subtraction as `A + NOT(B) + 1`), the logical ops map
/// gates bitwise, and [AluOp.not] inverts `a` while ignoring `b` entirely
/// (any content, any length — pinned by contract tests).
///
/// Shape rejection is this module's own [ArgumentError]; bit rejection is
/// inherited from the gates. The result is exactly as long as `a`.
library;

import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart'
    show AND, NOT, OR, XOR;

import 'adders.dart';

/// ALU operation codes.
enum AluOp {
  /// Addition: A + B.
  add,

  /// Subtraction: A - B (as A + NOT(B) + 1).
  sub,

  /// Bitwise AND.
  and,

  /// Bitwise OR.
  or,

  /// Bitwise XOR.
  xor,

  /// Bitwise NOT of A (B ignored).
  not,
}

/// Result of one ALU operation, with status flags.
class AluResult {
  /// Result bits, LSB first.
  final List<int> result;

  /// True iff every result bit is 0.
  final bool zero;

  /// For [AluOp.add]: carry out of the MSB (unsigned overflow). For
  /// [AluOp.sub]: set when NO borrow was needed (`a >= b` unsigned).
  /// Always false for the logical operations.
  final bool carry;

  /// True iff the MSB is 1 (negative in two's complement).
  final bool negative;

  /// True on signed overflow (wrong-sign result).
  final bool overflow;

  /// Creates a result.
  const AluResult({
    required this.result,
    required this.zero,
    required this.carry,
    required this.negative,
    required this.overflow,
  });

  @override
  bool operator ==(Object other) =>
      other is AluResult &&
      _bitsEqual(result, other.result) &&
      zero == other.zero &&
      carry == other.carry &&
      negative == other.negative &&
      overflow == other.overflow;

  @override
  int get hashCode =>
      Object.hash(Object.hashAll(result), zero, carry, negative, overflow);

  @override
  String toString() => 'AluResult(result: $result, zero: $zero, '
      'carry: $carry, negative: $negative, overflow: $overflow)';
}

bool _bitsEqual(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Executes [op] on N-bit LSB-first [a] and [b].
///
/// Throws [ArgumentError] when [a] is empty, when the operands differ in
/// length (except [AluOp.not], which never reads [b]), or when any read
/// bit is not 0 or 1.
AluResult alu(List<int> a, List<int> b, AluOp op) {
  if (a.isEmpty) {
    throw ArgumentError('a must not be empty');
  }
  if (op != AluOp.not && a.length != b.length) {
    throw ArgumentError(
        'a and b must have the same length, got ${a.length} and ${b.length}');
  }

  final List<int> result;
  final bool carry;
  switch (op) {
    case AluOp.add:
      final r = rippleCarryAdderWithCarry(a, b, 0);
      result = r.sum;
      carry = r.carryOut == 1;
    case AluOp.sub:
      final notB = [for (final bit in b) NOT(bit)];
      final r = rippleCarryAdderWithCarry(a, notB, 1);
      result = r.sum;
      carry = r.carryOut == 1;
    case AluOp.and:
      result = [for (var i = 0; i < a.length; i++) AND(a[i], b[i])];
      carry = false;
    case AluOp.or:
      result = [for (var i = 0; i < a.length; i++) OR(a[i], b[i])];
      carry = false;
    case AluOp.xor:
      result = [for (var i = 0; i < a.length; i++) XOR(a[i], b[i])];
      carry = false;
    case AluOp.not:
      result = [for (final bit in a) NOT(bit)];
      carry = false;
  }

  final zero = result.every((bit) => bit == 0);
  // Defensive: result is never empty here (a is not), mirroring the
  // reference's guard for the same unreachable shape.
  final negative = result.isNotEmpty && result.last == 1;

  var overflow = false;
  if (op == AluOp.add || op == AluOp.sub) {
    final aSign = a[a.length - 1];
    // Subtraction adds NOT(B), so its effective sign is inverted.
    final bSign = op == AluOp.add ? b[b.length - 1] : NOT(b[b.length - 1]);
    final rSign = result[result.length - 1];
    overflow = aSign == bSign && rSign != aSign;
  }

  return AluResult(
    result: result,
    zero: zero,
    carry: carry,
    negative: negative,
    overflow: overflow,
  );
}
