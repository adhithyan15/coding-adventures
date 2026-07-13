import 'gates.dart';

Bit mux2(Bit d0, Bit d1, Bit select) => mux(d0, d1, select);

Bit mux4(Bit d0, Bit d1, Bit d2, Bit d3, List<Bit> select) {
  _validateSelect(select, 2);
  final low = mux2(d0, d1, select[0]);
  final high = mux2(d2, d3, select[0]);
  return mux2(low, high, select[1]);
}

Bit mux8(List<Bit> inputs, List<Bit> select) {
  if (inputs.length != 8) {
    throw ArgumentError.value(inputs.length, 'inputs.length', 'must be 8');
  }
  return muxN(inputs, select);
}

/// Selects one of a power-of-two number of inputs using LSB-first select bits.
Bit muxN(List<Bit> inputs, List<Bit> select) {
  _requirePowerOfTwo(inputs.length, 'inputs.length');
  final expectedSelectBits = _log2(inputs.length);
  _validateSelect(select, expectedSelectBits);
  for (var index = 0; index < inputs.length; index++) {
    validateBit(inputs[index], 'inputs[$index]');
  }
  return inputs[_bitsToInt(select)];
}

/// Routes [data] to one output. The optional count must match the select width.
List<Bit> demux(Bit data, List<Bit> select, [int? outputCount]) {
  validateBit(data, 'data');
  for (var index = 0; index < select.length; index++) {
    validateBit(select[index], 'select[$index]');
  }
  final expectedCount = 1 << select.length;
  final count = outputCount ?? expectedCount;
  _requirePowerOfTwo(count, 'outputCount');
  if (count != expectedCount) {
    throw ArgumentError.value(
      count,
      'outputCount',
      'must equal 2^select.length ($expectedCount)',
    );
  }
  final active = _bitsToInt(select);
  return List<Bit>.generate(count, (index) => index == active ? data : 0);
}

List<Bit> demuxN(Bit data, List<Bit> select, int outputCount) =>
    demux(data, select, outputCount);

/// Converts an LSB-first binary value to a one-hot output.
List<Bit> decoder(List<Bit> inputs) {
  if (inputs.isEmpty) {
    throw ArgumentError.value(inputs, 'inputs', 'must not be empty');
  }
  for (var index = 0; index < inputs.length; index++) {
    validateBit(inputs[index], 'inputs[$index]');
  }
  final active = _bitsToInt(inputs);
  return List<Bit>.generate(
    1 << inputs.length,
    (index) => index == active ? 1 : 0,
  );
}

/// Encodes exactly one active input as an LSB-first binary index.
List<Bit> encoder(List<Bit> inputs) {
  _requirePowerOfTwo(inputs.length, 'inputs.length');
  var activeIndex = -1;
  for (var index = 0; index < inputs.length; index++) {
    validateBit(inputs[index], 'inputs[$index]');
    if (inputs[index] == 1) {
      if (activeIndex != -1) {
        throw ArgumentError.value(inputs, 'inputs', 'must be one-hot');
      }
      activeIndex = index;
    }
  }
  if (activeIndex == -1) {
    throw ArgumentError.value(inputs, 'inputs', 'must be one-hot');
  }
  return _intToBits(activeIndex, _log2(inputs.length));
}

/// Encodes the highest active input and returns a validity bit.
(List<Bit>, Bit) priorityEncoder(List<Bit> inputs) {
  _requirePowerOfTwo(inputs.length, 'inputs.length');
  var activeIndex = -1;
  for (var index = 0; index < inputs.length; index++) {
    validateBit(inputs[index], 'inputs[$index]');
    if (inputs[index] == 1) activeIndex = index;
  }
  final width = _log2(inputs.length);
  return activeIndex == -1
      ? (List<Bit>.filled(width, 0), 0)
      : (_intToBits(activeIndex, width), 1);
}

/// Returns null for the high-impedance state when disabled.
Bit? triState(Bit data, Bit enable) {
  validateBit(data, 'data');
  validateBit(enable, 'enable');
  return enable == 1 ? data : null;
}

void _validateSelect(List<Bit> select, int expectedLength) {
  if (select.length != expectedLength) {
    throw ArgumentError.value(
      select.length,
      'select.length',
      'must be $expectedLength',
    );
  }
  for (var index = 0; index < select.length; index++) {
    validateBit(select[index], 'select[$index]');
  }
}

void _requirePowerOfTwo(int value, String name) {
  if (value < 2 || value & (value - 1) != 0) {
    throw ArgumentError.value(value, name, 'must be a power of two >= 2');
  }
}

int _log2(int value) => value.bitLength - 1;

int _bitsToInt(List<Bit> bits) {
  var value = 0;
  for (var index = 0; index < bits.length; index++) {
    value |= bits[index] << index;
  }
  return value;
}

List<Bit> _intToBits(int value, int width) =>
    List<Bit>.generate(width, (index) => (value >> index) & 1);
