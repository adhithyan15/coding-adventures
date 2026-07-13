/// A binary signal. Public functions validate that values are exactly 0 or 1.
typedef Bit = int;

/// Rejects values outside the binary domain.
void validateBit(Bit value, [String name = 'input']) {
  if (value != 0 && value != 1) {
    throw ArgumentError.value(value, name, 'must be 0 or 1');
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

Bit NAND(Bit a, Bit b) => NOT(AND(a, b));

Bit NOR(Bit a, Bit b) => NOT(OR(a, b));

Bit XNOR(Bit a, Bit b) => NOT(XOR(a, b));

Bit nandNot(Bit a) => NAND(a, a);

Bit nandAnd(Bit a, Bit b) => nandNot(NAND(a, b));

Bit nandOr(Bit a, Bit b) => NAND(nandNot(a), nandNot(b));

Bit nandXor(Bit a, Bit b) {
  final nandAB = NAND(a, b);
  return NAND(NAND(a, nandAB), NAND(b, nandAB));
}

Bit nandNor(Bit a, Bit b) => nandNot(nandOr(a, b));

Bit nandXnor(Bit a, Bit b) => nandNot(nandXor(a, b));

Bit andN(List<Bit> inputs) {
  _requireAtLeastTwo(inputs, 'andN');
  return inputs.skip(1).fold(inputs.first, AND);
}

Bit orN(List<Bit> inputs) {
  _requireAtLeastTwo(inputs, 'orN');
  return inputs.skip(1).fold(inputs.first, OR);
}

/// Reduces bits as a parity tree. Empty input has even parity and returns 0.
Bit xorN(List<Bit> inputs) {
  for (var index = 0; index < inputs.length; index++) {
    validateBit(inputs[index], 'inputs[$index]');
  }
  if (inputs.isEmpty) return 0;
  return inputs.skip(1).fold(inputs.first, XOR);
}

void _requireAtLeastTwo(List<Bit> inputs, String operation) {
  if (inputs.length < 2) {
    throw ArgumentError.value(
      inputs,
      'inputs',
      '$operation requires at least 2 inputs',
    );
  }
  for (var index = 0; index < inputs.length; index++) {
    validateBit(inputs[index], 'inputs[$index]');
  }
}

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
