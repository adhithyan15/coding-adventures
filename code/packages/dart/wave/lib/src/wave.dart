import 'package:coding_adventures_trig/trig.dart' as trig;

/// A sinusoidal wave with validated amplitude and frequency.
final class Wave {
  /// Creates a wave with an optional phase offset in radians.
  Wave(this.amplitude, this.frequency, [this.phase = 0.0]) {
    if (!amplitude.isFinite || amplitude < 0.0) {
      throw ArgumentError.value(
        amplitude,
        'amplitude',
        'must be finite and non-negative',
      );
    }
    if (!frequency.isFinite || frequency <= 0.0) {
      throw ArgumentError.value(
        frequency,
        'frequency',
        'must be finite and positive',
      );
    }
    if (frequency > double.maxFinite / trig.twoPi) {
      throw ArgumentError.value(
        frequency,
        'frequency',
        'must keep angular frequency finite',
      );
    }
    if (!phase.isFinite) {
      throw ArgumentError.value(phase, 'phase', 'must be finite');
    }
  }

  /// Peak displacement from equilibrium.
  final double amplitude;

  /// Complete cycles per second.
  final double frequency;

  /// Initial phase offset in radians.
  final double phase;

  /// Duration of one complete cycle in seconds.
  double period() => 1.0 / frequency;

  /// Phase-change rate in radians per second.
  double angularFrequency() => 2.0 * trig.pi * frequency;

  /// Evaluates `amplitude * sin(2 * pi * frequency * time + phase)`.
  double evaluate(double time) {
    if (!time.isFinite) {
      throw ArgumentError.value(time, 'time', 'must be finite');
    }
    if (amplitude == 0.0) {
      return 0.0;
    }

    // Reduce time to one period before multiplying. This keeps the phase
    // finite even when two individually finite inputs would overflow if they
    // were multiplied directly.
    final representedPeriod = period();
    final reducedTime =
        representedPeriod.isInfinite ? time : time % representedPeriod;
    final reducedPhase = phase % trig.twoPi;
    final angle = trig.twoPi * (frequency * reducedTime) + reducedPhase;
    final unitValue = trig.sin(angle);
    if (unitValue >= 1.0) {
      return amplitude;
    }
    if (unitValue <= -1.0) {
      return -amplitude;
    }
    return amplitude * unitValue;
  }

  @override
  String toString() =>
      'Wave(amplitude: $amplitude, frequency: $frequency, phase: $phase)';
}
