import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart';
import 'package:test/test.dart';

void main() {
  group('latches and flip-flops', () {
    test('SR latch set, reset, hold, and invalid states', () {
      expect(srLatch(1, 0), (1, 0));
      expect(srLatch(0, 1), (0, 1));
      expect(srLatch(0, 0, q: 1, qBar: 0), (1, 0));
      expect(srLatch(0, 0, q: 0, qBar: 1), (0, 1));
      expect(srLatch(1, 1), (0, 0));
    });

    test('D latch is transparent only while enabled', () {
      expect(dLatch(1, 1), (1, 0));
      expect(dLatch(0, 1, q: 1, qBar: 0), (0, 1));
      expect(dLatch(0, 0, q: 1, qBar: 0), (1, 0));
    });

    test('D flip-flop captures on the low-to-high clock sequence', () {
      final low = dFlipFlop(1, 0);
      expect(low.$1, 0);
      expect(low.$3.masterQ, 1);
      final high = dFlipFlop(1, 1, state: low.$3);
      expect(high.$1, 1);
      expect(high.$2, 0);
    });

    test('invalid sequential signals are rejected', () {
      expect(() => srLatch(2, 0), throwsArgumentError);
      expect(() => dLatch(0, 2), throwsArgumentError);
      expect(() => dFlipFlop(0, -1), throwsArgumentError);
    });
  });

  group('register', () {
    test('captures a parallel word and later holds it', () {
      const data = [1, 0, 1, 1];
      final low = register(data, 0);
      final high = register(data, 1, state: low.$2);
      expect(high.$1, data);
      final hold = register([0, 0, 0, 0], 0, state: high.$2);
      expect(hold.$1, data);
    });

    test('validates data, width, and state lengths', () {
      expect(() => register([], 0), throwsArgumentError);
      expect(() => register([1, 2], 0), throwsArgumentError);
      expect(() => register([1, 0], 0, width: 3), throwsArgumentError);
      expect(
        () => register([1, 0], 0, state: [const FlipFlopState()]),
        throwsArgumentError,
      );
    });
  });

  group('shift register', () {
    test('shifts right and reports the outgoing bit', () {
      List<FlipFlopState>? state;
      for (final bit in [1, 0, 1, 1]) {
        final low = shiftRegister(bit, 0, state: state, width: 4);
        final high = shiftRegister(bit, 1, state: low.$3, width: 4);
        state = high.$3;
      }
      expect(state!.map((item) => item.slaveQ).toList(), [1, 1, 0, 1]);
      final low = shiftRegister(0, 0, state: state, width: 4);
      final high = shiftRegister(0, 1, state: low.$3, width: 4);
      expect(high.$2, 1);
      expect(high.$1, [0, 1, 1, 0]);
    });

    test('shifts left', () {
      final low = shiftRegister(1, 0, width: 4, direction: 'left');
      final high = shiftRegister(
        1,
        1,
        state: low.$3,
        width: 4,
        direction: 'left',
      );
      expect(high.$1, [0, 0, 0, 1]);
      expect(high.$2, 0);
    });

    test('validates direction, width, and state size', () {
      expect(() => shiftRegister(0, 0, width: 0), throwsArgumentError);
      expect(() => shiftRegister(0, 0, direction: 'up'), throwsArgumentError);
      expect(
        () => shiftRegister(0, 0, state: [const FlipFlopState()], width: 2),
        throwsArgumentError,
      );
    });
  });

  group('counter', () {
    test('counts LSB-first, resets, and wraps', () {
      CounterState? state;
      final values = <List<Bit>>[];
      for (var tick = 0; tick < 4; tick++) {
        final low = counter(0, state: state, width: 3);
        final high = counter(1, state: low.$2, width: 3);
        state = high.$2;
        values.add(high.$1);
      }
      expect(values, [
        [1, 0, 0],
        [0, 1, 0],
        [1, 1, 0],
        [0, 0, 1],
      ]);

      final resetLow = counter(0, reset: 1, state: state, width: 3);
      final resetHigh = counter(1, reset: 1, state: resetLow.$2, width: 3);
      expect(resetHigh.$1, [0, 0, 0]);

      state = null;
      for (var tick = 0; tick < 8; tick++) {
        final low = counter(0, state: state, width: 3);
        final high = counter(1, state: low.$2, width: 3);
        state = high.$2;
      }
      expect(state!.value, [0, 0, 0]);
    });

    test('clock low holds the externally stored value', () {
      final state = CounterState(
        value: [1, 0],
        ffState: const [FlipFlopState(), FlipFlopState()],
      );
      final result = counter(0, state: state, width: 2);
      expect(result.$2.value, [1, 0]);
    });

    test('validates width and state shape', () {
      expect(() => counter(0, width: 0), throwsArgumentError);
      expect(
        () => counter(
          0,
          width: 2,
          state: CounterState(value: [0], ffState: const [FlipFlopState()]),
        ),
        throwsArgumentError,
      );
    });
  });
}
