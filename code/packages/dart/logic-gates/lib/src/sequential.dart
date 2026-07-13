import 'gates.dart';

/// Complete internal state of one master-slave D flip-flop.
class FlipFlopState {
  const FlipFlopState({
    this.masterQ = 0,
    this.masterQBar = 1,
    this.slaveQ = 0,
    this.slaveQBar = 1,
  });

  final Bit masterQ;
  final Bit masterQBar;
  final Bit slaveQ;
  final Bit slaveQBar;

  @override
  bool operator ==(Object other) =>
      other is FlipFlopState &&
      masterQ == other.masterQ &&
      masterQBar == other.masterQBar &&
      slaveQ == other.slaveQ &&
      slaveQBar == other.slaveQBar;

  @override
  int get hashCode => Object.hash(masterQ, masterQBar, slaveQ, slaveQBar);
}

/// Stored counter value and the backing flip-flop states.
class CounterState {
  CounterState({required List<Bit> value, required List<FlipFlopState> ffState})
      : value = List<Bit>.unmodifiable(value),
        ffState = List<FlipFlopState>.unmodifiable(ffState);

  final List<Bit> value;
  final List<FlipFlopState> ffState;
}

/// Evaluates a cross-coupled NOR SR latch until it stabilizes.
(Bit, Bit) srLatch(Bit set, Bit reset, {Bit q = 0, Bit qBar = 1}) {
  validateBit(set, 'set');
  validateBit(reset, 'reset');
  validateBit(q, 'q');
  validateBit(qBar, 'qBar');
  for (var iteration = 0; iteration < 10; iteration++) {
    final newQ = NOR(reset, qBar);
    final newQBar = NOR(set, q);
    if (newQ == q && newQBar == qBar) break;
    q = newQ;
    qBar = newQBar;
  }
  return (q, qBar);
}

(Bit, Bit) dLatch(Bit data, Bit enable, {Bit q = 0, Bit qBar = 1}) {
  validateBit(data, 'data');
  validateBit(enable, 'enable');
  validateBit(q, 'q');
  validateBit(qBar, 'qBar');
  return srLatch(AND(data, enable), AND(NOT(data), enable), q: q, qBar: qBar);
}

/// Evaluates one phase of an edge-triggered master-slave D flip-flop.
(Bit, Bit, FlipFlopState) dFlipFlop(
  Bit data,
  Bit clock, {
  FlipFlopState? state,
}) {
  validateBit(data, 'data');
  validateBit(clock, 'clock');
  final current = state ?? const FlipFlopState();
  _validateFlipFlopState(current);
  final master = dLatch(
    data,
    NOT(clock),
    q: current.masterQ,
    qBar: current.masterQBar,
  );
  final slave = dLatch(
    master.$1,
    clock,
    q: current.slaveQ,
    qBar: current.slaveQBar,
  );
  final nextState = FlipFlopState(
    masterQ: master.$1,
    masterQBar: master.$2,
    slaveQ: slave.$1,
    slaveQBar: slave.$2,
  );
  return (slave.$1, slave.$2, nextState);
}

/// Evaluates one clock phase of an N-bit parallel register.
(List<Bit>, List<FlipFlopState>) register(
  List<Bit> data,
  Bit clock, {
  List<FlipFlopState>? state,
  int? width,
}) {
  validateBit(clock, 'clock');
  if (data.isEmpty) {
    throw ArgumentError.value(data, 'data', 'must not be empty');
  }
  if (width != null && data.length != width) {
    throw ArgumentError.value(
      data.length,
      'data.length',
      'must match width $width',
    );
  }
  for (var index = 0; index < data.length; index++) {
    validateBit(data[index], 'data[$index]');
  }
  final current =
      state ?? List<FlipFlopState>.filled(data.length, const FlipFlopState());
  if (current.length != data.length) {
    throw ArgumentError.value(
      current.length,
      'state.length',
      'must match data length ${data.length}',
    );
  }
  final output = <Bit>[];
  final nextState = <FlipFlopState>[];
  for (var index = 0; index < data.length; index++) {
    final result = dFlipFlop(data[index], clock, state: current[index]);
    output.add(result.$1);
    nextState.add(result.$3);
  }
  return (
    List<Bit>.unmodifiable(output),
    List<FlipFlopState>.unmodifiable(nextState),
  );
}

/// Evaluates one clock phase of a serial-to-parallel shift register.
(List<Bit>, Bit, List<FlipFlopState>) shiftRegister(
  Bit serialIn,
  Bit clock, {
  List<FlipFlopState>? state,
  int width = 8,
  String direction = 'right',
}) {
  validateBit(serialIn, 'serialIn');
  validateBit(clock, 'clock');
  if (width < 1) {
    throw ArgumentError.value(width, 'width', 'must be >= 1');
  }
  if (direction != 'right' && direction != 'left') {
    throw ArgumentError.value(
      direction,
      'direction',
      "must be 'right' or 'left'",
    );
  }
  final current =
      state ?? List<FlipFlopState>.filled(width, const FlipFlopState());
  if (current.length != width) {
    throw ArgumentError.value(
      current.length,
      'state.length',
      'must match width $width',
    );
  }
  final values = current.map((item) => item.slaveQ).toList(growable: false);
  final serialOut = direction == 'right' ? values.last : values.first;
  final inputs = direction == 'right'
      ? <Bit>[serialIn, ...values.take(width - 1)]
      : <Bit>[...values.skip(1), serialIn];
  final result = register(inputs, clock, state: current, width: width);
  return (result.$1, serialOut, result.$2);
}

/// Evaluates one clock phase of an LSB-first binary counter.
(List<Bit>, CounterState) counter(
  Bit clock, {
  Bit reset = 0,
  CounterState? state,
  int width = 8,
}) {
  validateBit(clock, 'clock');
  validateBit(reset, 'reset');
  if (width < 1) {
    throw ArgumentError.value(width, 'width', 'must be >= 1');
  }
  final current = state ??
      CounterState(
        value: List<Bit>.filled(width, 0),
        ffState: List<FlipFlopState>.filled(width, const FlipFlopState()),
      );
  if (current.value.length != width || current.ffState.length != width) {
    throw ArgumentError('counter state lengths must match width $width');
  }
  for (var index = 0; index < width; index++) {
    validateBit(current.value[index], 'state.value[$index]');
    _validateFlipFlopState(current.ffState[index]);
  }
  final nextValue =
      reset == 1 ? List<Bit>.filled(width, 0) : _increment(current.value);
  final result = register(
    nextValue,
    clock,
    state: current.ffState,
    width: width,
  );
  final storedValue = clock == 1 ? nextValue : current.value;
  return (result.$1, CounterState(value: storedValue, ffState: result.$2));
}

List<Bit> _increment(List<Bit> value) {
  var carry = 1;
  final output = <Bit>[];
  for (final bit in value) {
    output.add(XOR(bit, carry));
    carry = AND(bit, carry);
  }
  return output;
}

void _validateFlipFlopState(FlipFlopState state) {
  validateBit(state.masterQ, 'state.masterQ');
  validateBit(state.masterQBar, 'state.masterQBar');
  validateBit(state.slaveQ, 'state.slaveQ');
  validateBit(state.slaveQBar, 'state.slaveQBar');
}
