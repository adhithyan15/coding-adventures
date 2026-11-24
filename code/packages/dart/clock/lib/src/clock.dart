/// Square-wave clock generator with edges, dividers, and phase clocks.
///
/// The Dart lane of the Rust `clock` crate: [Clock] alternates between 0
/// and 1, emitting a [ClockEdge] per half-cycle to registered listeners;
/// [ClockDivider] turns source rising edges into a slower output clock; and
/// [MultiPhaseClock] rotates one active phase per source rising edge.
///
/// This is a pure simulation: simulation time only, no wall-clock reads.
/// The reference's `assert!` panics become [ArgumentError]s (Dart asserts
/// are debug-only, so API validation must throw unconditionally), with the
/// same messages; out-of-range phase indexing throws [RangeError] via the
/// list access itself, as in the reference.
library;

/// Record of one clock transition: which cycle, the new level, and the
/// edge direction. Cycles count from 1; the first tick is always rising.
class ClockEdge {
  /// Cycle number (incremented on each rising edge).
  final int cycle;

  /// Signal level after the transition (0 or 1).
  final int value;

  /// True for 0→1 transitions.
  final bool isRising;

  /// True for 1→0 transitions.
  final bool isFalling;

  /// Creates an edge record.
  const ClockEdge({
    required this.cycle,
    required this.value,
    required this.isRising,
    required this.isFalling,
  });

  @override
  bool operator ==(Object other) =>
      other is ClockEdge &&
      cycle == other.cycle &&
      value == other.value &&
      isRising == other.isRising &&
      isFalling == other.isFalling;

  @override
  int get hashCode => Object.hash(cycle, value, isRising, isFalling);

  @override
  String toString() => 'ClockEdge(cycle: $cycle, value: $value, '
      'isRising: $isRising, isFalling: $isFalling)';
}

/// System clock generator: a toggle switch alternating 0 → 1 → 0.
///
/// A complete cycle is two ticks (rising then falling). Listeners registered
/// via [registerListener] observe every edge, in registration order.
class Clock {
  /// The clock frequency in Hz.
  final int frequencyHz;

  int _cycle = 0;
  int _value = 0;
  int _totalTicks = 0;
  final List<void Function(ClockEdge edge)> _listeners = [];

  /// Creates a clock at [frequencyHz], starting at value 0, cycle 0.
  ///
  /// Throws [ArgumentError] when [frequencyHz] is not positive.
  Clock(this.frequencyHz) {
    if (frequencyHz <= 0) {
      throw ArgumentError('frequency_hz must be > 0');
    }
  }

  /// Current cycle number (incremented on each rising edge).
  int get cycle => _cycle;

  /// Current signal value (0 or 1).
  int get value => _value;

  /// Total half-cycles since creation or the last [reset].
  int get totalTicks => _totalTicks;

  /// Advances one half-cycle, returning the edge that occurred.
  ///
  /// From 0 the clock rises (starting a new cycle); from 1 it falls.
  /// Listeners run after the state updates, in registration order.
  ClockEdge tick() {
    final oldValue = _value;
    _value = 1 - _value;
    _totalTicks++;

    final isRising = oldValue == 0 && _value == 1;
    final isFalling = oldValue == 1 && _value == 0;
    if (isRising) _cycle++;

    final edge = ClockEdge(
      cycle: _cycle,
      value: _value,
      isRising: isRising,
      isFalling: isFalling,
    );
    for (final listener in _listeners) {
      listener(edge);
    }
    return edge;
  }

  /// Runs one complete cycle (rising + falling edge).
  (ClockEdge, ClockEdge) fullCycle() => (tick(), tick());

  /// Runs [cycles] complete cycles, returning all 2N edges in order.
  List<ClockEdge> run(int cycles) {
    final edges = <ClockEdge>[];
    for (var i = 0; i < cycles; i++) {
      final (rising, falling) = fullCycle();
      edges.add(rising);
      edges.add(falling);
    }
    return edges;
  }

  /// Registers [listener] for every future edge.
  void registerListener(void Function(ClockEdge edge) listener) {
    _listeners.add(listener);
  }

  /// Resets value, cycle, and tick counts to zero.
  ///
  /// Listeners are preserved — only the timing state resets.
  void reset() {
    _cycle = 0;
    _value = 0;
    _totalTicks = 0;
  }

  /// Clock period in nanoseconds: `1e9 / frequency_hz`.
  double periodNs() => 1e9 / frequencyHz;
}

/// Divides a source frequency by an integer factor.
///
/// Counts source rising edges via [onEdge]; every [divisor]-th rising edge
/// produces one complete cycle on [output]. Output frequency equals
/// `source ~/ divisor`.
class ClockDivider {
  /// Rising edges per output cycle.
  final int divisor;

  int _counter = 0;
  late final Clock _output;

  /// Creates a divider for a [sourceFrequencyHz] source.
  ///
  /// Throws [ArgumentError] when [divisor] is less than 2. The divisor is
  /// validated before the output clock is built, so a bad divisor reports
  /// itself even when the divided frequency would also be invalid.
  ClockDivider(int sourceFrequencyHz, this.divisor) {
    if (divisor < 2) {
      throw ArgumentError('Divisor must be >= 2, got $divisor');
    }
    _output = Clock(sourceFrequencyHz ~/ divisor);
  }

  /// The divided output clock (mutable: listeners may register on it).
  Clock get output => _output;

  /// Feeds one source edge in. Rising edges advance the counter; every
  /// [divisor]-th one ticks the output twice (rising + falling).
  void onEdge(ClockEdge edge) {
    if (edge.isRising) {
      _counter++;
      if (_counter >= divisor) {
        _counter = 0;
        _output.tick();
        _output.tick();
      }
    }
  }
}

/// Generates [numPhases] non-overlapping clock phases from one source.
///
/// On each source rising edge, exactly one phase reads 1 and the rest 0;
/// the active phase rotates 0 → 1 → … → N-1 → 0. Before the first rising
/// edge every phase reads 0.
class MultiPhaseClock {
  /// Number of phases.
  final int numPhases;

  int _activePhase = 0;
  late List<int> _phaseValues;

  /// Creates a clock with [numPhases] phases, all initially 0.
  ///
  /// Throws [ArgumentError] when [numPhases] is less than 2. The count is
  /// validated before allocation, so a bad count reports itself.
  MultiPhaseClock(this.numPhases) {
    if (numPhases < 2) {
      throw ArgumentError('Phases must be >= 2, got $numPhases');
    }
    _phaseValues = List<int>.filled(numPhases, 0);
  }

  /// Feeds one source edge in. Rising edges rotate the active phase;
  /// falling edges are ignored.
  void onEdge(ClockEdge edge) {
    if (edge.isRising) {
      _phaseValues = List<int>.filled(numPhases, 0);
      _phaseValues[_activePhase] = 1;
      _activePhase = (_activePhase + 1) % numPhases;
    }
  }

  /// Current value (0 or 1) of phase [index].
  ///
  /// Throws [RangeError] when [index] is out of range.
  int getPhase(int index) => _phaseValues[index];
}
