import 'gates.dart';
import 'internal.dart';

Bit mux2(Bit d0, Bit d1, Bit select) => mux(d0, d1, select);

Bit mux4(Bit d0, Bit d1, Bit d2, Bit d3, List<Bit> select) {
  final sel = _snapshotSelect(select, 'select', exactly: 2);
  final low = mux2(d0, d1, sel[0]);
  final high = mux2(d2, d3, sel[0]);
  return mux2(low, high, sel[1]);
}

Bit mux8(List<Bit> inputs, List<Bit> select) {
  final data = snapshotBits(inputs, 'inputs', max: 8);
  if (data.length != 8) {
    throw ArgumentError.value(data.length, 'inputs.length', 'must be 8');
  }
  return muxN(data, select);
}

/// Selects one of a power-of-two number of inputs using LSB-first select bits.
Bit muxN(List<Bit> inputs, List<Bit> select) {
  final data = snapshotBits(inputs, 'inputs');
  _requirePowerOfTwo(data.length, 'inputs.length');
  final sel = _snapshotSelect(select, 'select', exactly: _log2(data.length));
  return data[_bitsToInt(sel)];
}

/// Routes [data] to one output. The optional count must match the select width.
List<Bit> demux(Bit data, List<Bit> select, [int? outputCount]) {
  validateBit(data, 'data');
  final bit = data == 1 ? 1 : 0; // canonical int (web: 1.0 / -0.0)
  final sel = snapshotBits(select, 'select', max: maxSelectBits);
  final expectedCount = 1 << sel.length; // safe: length <= 16
  final count = outputCount ?? expectedCount;
  _requirePowerOfTwo(count, 'outputCount');
  if (count != expectedCount) {
    throw ArgumentError.value(
      count,
      'outputCount',
      'must equal 2^select.length ($expectedCount)',
    );
  }
  final active = _bitsToInt(sel);
  return List<Bit>.generate(count, (index) => index == active ? bit : 0);
}

List<Bit> demuxN(Bit data, List<Bit> select, int outputCount) =>
    demux(data, select, outputCount);

/// Converts an LSB-first binary value to a one-hot output.
List<Bit> decoder(List<Bit> inputs) {
  final bits = snapshotBits(
    inputs,
    'inputs',
    max: maxSelectBits,
    min: 1,
    shape: 'must not be empty',
  );
  final active = _bitsToInt(bits);
  return List<Bit>.generate(
    1 << bits.length, // safe: length <= 16
    (index) => index == active ? 1 : 0,
  );
}

/// Encodes exactly one active input as an LSB-first binary index.
List<Bit> encoder(List<Bit> inputs) {
  final bits = snapshotBits(inputs, 'inputs');
  _requirePowerOfTwo(bits.length, 'inputs.length');
  var activeIndex = -1;
  for (var index = 0; index < bits.length; index++) {
    if (bits[index] == 1) {
      if (activeIndex != -1) {
        throw ArgumentError('inputs must be one-hot');
      }
      activeIndex = index;
    }
  }
  if (activeIndex == -1) {
    throw ArgumentError('inputs must be one-hot');
  }
  return _intToBits(activeIndex, _log2(bits.length));
}

/// Encodes the highest active input and returns a validity bit.
(List<Bit>, Bit) priorityEncoder(List<Bit> inputs) {
  final bits = snapshotBits(inputs, 'inputs');
  _requirePowerOfTwo(bits.length, 'inputs.length');
  var activeIndex = -1;
  for (var index = 0; index < bits.length; index++) {
    if (bits[index] == 1) activeIndex = index;
  }
  final width = _log2(bits.length);
  return activeIndex == -1
      ? (List<Bit>.filled(width, 0), 0)
      : (_intToBits(activeIndex, width), 1);
}

/// Returns null for the high-impedance state when disabled.
Bit? triState(Bit data, Bit enable) {
  validateBit(data, 'data');
  validateBit(enable, 'enable');
  return enable == 1 ? (data == 1 ? 1 : 0) : null;
}

List<Bit> _snapshotSelect(
  List<Bit> select,
  String name, {
  required int exactly,
}) {
  final sel = snapshotBits(select, name, max: maxSelectBits);
  if (sel.length != exactly) {
    throw ArgumentError.value(sel.length, '$name.length', 'must be $exactly');
  }
  return sel;
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
