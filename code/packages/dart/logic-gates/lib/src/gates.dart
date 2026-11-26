import 'internal.dart';

/// A binary signal. Public functions validate that values are exactly 0 or 1.
typedef Bit = int;

/// Thrown when a gate argument is not exactly the integer 0 or 1.
///
/// This is an [Error], not an [Exception]: `on Exception` will not catch it.
///
/// It extends [ArgumentError], so `on ArgumentError` handlers keep working,
/// and exposes the standard [name] (the offending parameter, as spelled by the
/// public gate the caller invoked) and [invalidValue] (the rejected integer).
///
/// The constructor accepts any object but never calls caller code on it: an
/// `int` is kept as [invalidValue]; anything else is replaced by the fixed text
/// `'non-integer value'` so messages cannot run a caller `toString`.
class InvalidBitError extends ArgumentError {
  InvalidBitError(Object? value, String name)
      : super.value(
          value is int ? value : 'non-integer value',
          name,
          'must be 0 or 1',
        );
}

/// Rejects values outside the binary domain.
///
/// Web limitation: dart2js represents ints as JS numbers, so `1.0` and `-0.0`
/// are indistinguishable from `1` and `0` at runtime and are accepted as bits
/// (they are numerically exactly 0 or 1). List snapshots canonicalise them.
///
/// Dart's static types already stop non-`int` arguments at compile time; a
/// `dynamic` caller passing a double, bool, String, or null hits a TypeError at
/// the call boundary before this function runs (on the Dart VM, the tested
/// platform; on the web `1.0 is int` is true, so a JS caller could pass 1.0).
void validateBit(Object? value, [String name = 'input']) {
  if (value is! int || (value != 0 && value != 1)) {
    throw InvalidBitError(value, name);
  }
}

Bit NOT(Bit a) {
  validateBit(a, 'a');
  return 1 - a;
}

Bit AND(Bit a, Bit b) {
  validateBit(a, 'a');
  validateBit(b, 'b');
  return a == 1 && b == 1 ? 1 : 0;
}

Bit OR(Bit a, Bit b) {
  validateBit(a, 'a');
  validateBit(b, 'b');
  return a == 1 || b == 1 ? 1 : 0;
}

Bit XOR(Bit a, Bit b) {
  validateBit(a, 'a');
  validateBit(b, 'b');
  return a == b ? 0 : 1;
}

/// Validates both operands under the caller's own parameter names, so the
/// NAND-built gates never leak an inner helper's parameter name.
void _validate2(Bit a, Bit b) {
  validateBit(a, 'a');
  validateBit(b, 'b');
}

Bit NAND(Bit a, Bit b) {
  _validate2(a, b);
  return NOT(AND(a, b));
}

Bit NOR(Bit a, Bit b) {
  _validate2(a, b);
  return NOT(OR(a, b));
}

Bit XNOR(Bit a, Bit b) {
  _validate2(a, b);
  return NOT(XOR(a, b));
}

Bit nandNot(Bit a) {
  validateBit(a, 'a');
  return NAND(a, a);
}

Bit nandAnd(Bit a, Bit b) {
  _validate2(a, b);
  return nandNot(NAND(a, b));
}

Bit nandOr(Bit a, Bit b) {
  _validate2(a, b);
  return NAND(nandNot(a), nandNot(b));
}

Bit nandXor(Bit a, Bit b) {
  _validate2(a, b);
  final nandAB = NAND(a, b);
  return NAND(NAND(a, nandAB), NAND(b, nandAB));
}

Bit nandNor(Bit a, Bit b) {
  _validate2(a, b);
  return nandNot(nandOr(a, b));
}

Bit nandXnor(Bit a, Bit b) {
  _validate2(a, b);
  return nandNot(nandXor(a, b));
}

Bit andN(List<Bit> inputs) {
  final bits = _snapshotAtLeastTwo(inputs, 'andN');
  return bits.skip(1).fold(bits.first, AND);
}

Bit orN(List<Bit> inputs) {
  final bits = _snapshotAtLeastTwo(inputs, 'orN');
  return bits.skip(1).fold(bits.first, OR);
}

/// Reduces bits as a parity tree. Empty input has even parity and returns 0.
Bit xorN(List<Bit> inputs) {
  final bits = snapshotBits(inputs, 'inputs');
  if (bits.isEmpty) return 0;
  return bits.skip(1).fold(bits.first, XOR);
}

List<Bit> _snapshotAtLeastTwo(List<Bit> inputs, String operation) =>
    snapshotBits(
      inputs,
      'inputs',
      min: 2,
      shape: '$operation requires at least 2 inputs',
    );

/// Selects `a` when [select] is 0 and `b` when it is 1.
Bit mux(Bit a, Bit b, Bit select) {
  validateBit(a, 'a');
  validateBit(b, 'b');
  validateBit(select, 'select');
  return OR(AND(a, NOT(select)), AND(b, select));
}

/// Routes [input] to output zero or one according to [select].
(Bit, Bit) dmux(Bit input, Bit select) {
  validateBit(input, 'input');
  validateBit(select, 'select');
  return (AND(input, NOT(select)), AND(input, select));
}
