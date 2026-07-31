import 'package:coding_adventures_trig/trig.dart' as trig;
import 'package:coding_adventures_wave/wave.dart';
import 'package:test/test.dart';

const tolerance = 1e-10;

Matcher closeToExpected(double expected) => closeTo(expected, tolerance);

void main() {
  group('construction', () {
    test('defaults phase to zero and preserves explicit parameters', () {
      final defaultPhase = Wave(1.0, 1.0);
      expect(defaultPhase.phase, 0.0);

      final wave = Wave(3.5, 2.0, 1.25);
      expect(wave.amplitude, 3.5);
      expect(wave.frequency, 2.0);
      expect(wave.phase, 1.25);
    });

    test('allows a zero-amplitude flat wave', () {
      final wave = Wave(0.0, 1.0);
      expect(wave.amplitude, 0.0);
    });

    test('has a stable inspectable representation', () {
      expect(
        Wave(2.0, 3.0, 0.5).toString(),
        'Wave(amplitude: 2.0, frequency: 3.0, phase: 0.5)',
      );
    });
  });

  group('validation', () {
    test('rejects a negative amplitude', () {
      expect(() => Wave(-1.0, 1.0), throwsArgumentError);
    });

    test('rejects zero and negative frequencies', () {
      expect(() => Wave(1.0, 0.0), throwsArgumentError);
      expect(() => Wave(1.0, -5.0), throwsArgumentError);
    });

    test('rejects non-finite parameters and angular-frequency overflow', () {
      for (final amplitude in [double.nan, double.infinity]) {
        expect(() => Wave(amplitude, 1.0), throwsArgumentError);
      }
      for (final frequency in [double.nan, double.infinity, double.maxFinite]) {
        expect(() => Wave(1.0, frequency), throwsArgumentError);
      }
      for (final phase in [double.nan, double.infinity]) {
        expect(() => Wave(1.0, 1.0, phase), throwsArgumentError);
      }
    });
  });

  group('derived quantities', () {
    test('computes period and angular frequency', () {
      expect(Wave(1.0, 4.0).period(), closeToExpected(0.25));
      expect(
        Wave(1.0, 10.0).angularFrequency(),
        closeToExpected(2.0 * trig.pi * 10.0),
      );
    });
  });

  group('evaluate', () {
    test('covers zero, peak, zero crossing, and trough', () {
      final wave = Wave(1.0, 1.0);
      expect(wave.evaluate(0.0), closeToExpected(0.0));
      expect(wave.evaluate(0.25), closeToExpected(1.0));
      expect(wave.evaluate(0.5), closeToExpected(0.0));
      expect(wave.evaluate(0.75), closeToExpected(-1.0));
    });

    test('is periodic', () {
      final wave = Wave(2.5, 3.0, 0.7);
      const time = 0.137;
      expect(
        wave.evaluate(time),
        closeToExpected(wave.evaluate(time + wave.period())),
      );
    });

    test('respects phase offsets', () {
      expect(
        Wave(1.0, 1.0, trig.halfPi).evaluate(0.0),
        closeToExpected(1.0),
      );
      expect(
        Wave(1.0, 1.0, 3.0 * trig.halfPi).evaluate(0.0),
        closeToExpected(-1.0),
      );
    });

    test('keeps a zero-amplitude wave flat', () {
      final wave = Wave(0.0, 1.0, trig.halfPi);
      for (final time in [0.0, 0.25, 0.5, 0.75, 1.0]) {
        expect(wave.evaluate(time), closeToExpected(0.0));
      }
    });

    test('rejects non-finite time', () {
      final wave = Wave(1.0, 1.0);
      expect(() => wave.evaluate(double.nan), throwsArgumentError);
      expect(() => wave.evaluate(double.infinity), throwsArgumentError);
    });

    test('keeps extreme finite evaluation bounded', () {
      final flat = Wave(0.0, 1e300, trig.halfPi);
      expect(flat.evaluate(double.maxFinite), 0.0);

      final extreme = Wave(double.maxFinite, 1e300, trig.halfPi);
      expect(extreme.evaluate(double.maxFinite).isFinite, isTrue);
    });
  });
}
