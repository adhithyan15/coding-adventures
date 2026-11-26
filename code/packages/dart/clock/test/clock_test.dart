/// Unit tests for `coding_adventures_clock`.
///
/// Mirrors the Rust reference's inline suite, then covers what it leaves
/// implicit: listener ordering and reset survival, divider ratios and
/// output wiring, multiphase rotation and bounds, edge value semantics,
/// and exact validation messages.
library;

import 'package:coding_adventures_clock/coding_adventures_clock.dart';
import 'package:test/test.dart';

void main() {
  group('mirror: clock basics', () {
    test('new clock starts at rest', () {
      final clk = Clock(1000000);
      expect(clk.frequencyHz, 1000000);
      expect(clk.cycle, 0);
      expect(clk.value, 0);
      expect(clk.totalTicks, 0);
    });

    test('first tick rises', () {
      final clk = Clock(1000000);
      final edge = clk.tick();
      expect(edge.isRising, isTrue);
      expect(edge.isFalling, isFalse);
      expect(edge.value, 1);
      expect(edge.cycle, 1);
    });

    test('second tick falls', () {
      final clk = Clock(1000000);
      clk.tick();
      final edge = clk.tick();
      expect(edge.isRising, isFalse);
      expect(edge.isFalling, isTrue);
      expect(edge.value, 0);
      expect(edge.cycle, 1);
    });

    test('full cycle ticks twice', () {
      final clk = Clock(1000000);
      final (rising, falling) = clk.fullCycle();
      expect(rising.isRising, isTrue);
      expect(falling.isFalling, isTrue);
      expect(clk.cycle, 1);
      expect(clk.totalTicks, 2);
    });

    test('run produces two edges per cycle', () {
      final clk = Clock(1000000);
      final edges = clk.run(5);
      expect(edges, hasLength(10));
      expect(clk.cycle, 5);
      expect(clk.totalTicks, 10);
    });

    test('period is inverse frequency', () {
      final clk = Clock(1000000000);
      expect((clk.periodNs() - 1.0).abs(), lessThan(0.001));
    });

    test('reset restores rest state', () {
      final clk = Clock(1000000);
      clk.run(5);
      clk.reset();
      expect(clk.cycle, 0);
      expect(clk.value, 0);
      expect(clk.totalTicks, 0);
    });

    test('listeners observe rising edges', () {
      var count = 0;
      final clk = Clock(1000000);
      clk.registerListener((edge) {
        if (edge.isRising) count++;
      });
      clk.run(3);
      expect(count, 3);
    });

    test('zero frequency throws', () {
      expect(() => Clock(0), throwsArgumentError);
    });

    test('small divisor throws', () {
      expect(() => ClockDivider(1000000, 1), throwsArgumentError);
    });

    test('small phase count throws', () {
      expect(() => MultiPhaseClock(1), throwsArgumentError);
    });
  });

  group('EXTRA: listeners', () {
    test('listeners run in registration order on every edge', () {
      final seen = <String>[];
      final clk = Clock(100);
      clk.registerListener((edge) => seen.add('first:${edge.value}'));
      clk.registerListener((edge) => seen.add('second:${edge.value}'));
      clk.tick();
      clk.tick();
      expect(seen, ['first:1', 'second:1', 'first:0', 'second:0']);
    });

    test('reset preserves listeners', () {
      var count = 0;
      final clk = Clock(100);
      clk.registerListener((_) => count++);
      clk.tick();
      clk.reset();
      clk.tick();
      expect(count, 2);
      expect(clk.totalTicks, 1);
    });
  });

  group('EXTRA: divider behavior', () {
    test('output frequency is the integer quotient', () {
      final divider = ClockDivider(1000000000, 4);
      expect(divider.divisor, 4);
      expect(divider.output.frequencyHz, 250000000);
    });

    test('output cycles after divisor rising edges', () {
      final source = Clock(100);
      final divider = ClockDivider(100, 4);
      for (var i = 0; i < 7; i++) {
        divider.onEdge(source.tick());
      }
      // Seven source ticks hold four rising edges: one output cycle.
      expect(divider.output.cycle, 1);
      expect(divider.output.totalTicks, 2);
      expect(divider.output.value, 0);
      divider.onEdge(source.tick());
      expect(divider.output.cycle, 1);
    });

    test('falling source edges never advance the divider', () {
      final divider = ClockDivider(100, 2);
      const falling =
          ClockEdge(cycle: 1, value: 0, isRising: false, isFalling: true);
      divider.onEdge(falling);
      divider.onEdge(falling);
      expect(divider.output.totalTicks, 0);
    });

    test('output clock accepts its own listeners', () {
      final source = Clock(100);
      final divider = ClockDivider(100, 2);
      var rising = 0;
      divider.output.registerListener((edge) {
        if (edge.isRising) rising++;
      });
      for (var i = 0; i < 4; i++) {
        divider.onEdge(source.tick());
      }
      expect(rising, 1);
    });

    test('divisor validation precedes output construction', () {
      // A bad divisor reports itself even when the quotient frequency
      // would also be invalid.
      expect(
          () => ClockDivider(0, 1),
          throwsA(isA<ArgumentError>().having(
              (e) => e.message, 'message', contains('Divisor must be >= 2'))));
    });
  });

  group('EXTRA: multiphase behavior', () {
    test('phases start at zero and rotate on rising edges', () {
      final source = Clock(100);
      final mpc = MultiPhaseClock(4);
      expect(mpc.numPhases, 4);
      for (var p = 0; p < 4; p++) {
        expect(mpc.getPhase(p), 0);
      }
      final seen = <List<int>>[];
      for (var i = 0; i < 4; i++) {
        mpc.onEdge(source.tick()); // rising
        mpc.onEdge(source.tick()); // falling (ignored)
        seen.add([for (var p = 0; p < 4; p++) mpc.getPhase(p)]);
      }
      expect(seen, [
        [1, 0, 0, 0],
        [0, 1, 0, 0],
        [0, 0, 1, 0],
        [0, 0, 0, 1],
      ]);
      // Fifth rising edge wraps back to phase 0.
      mpc.onEdge(source.tick());
      expect(mpc.getPhase(0), 1);
      expect(mpc.getPhase(3), 0);
    });

    test('falling edges change nothing', () {
      final mpc = MultiPhaseClock(3);
      const falling =
          ClockEdge(cycle: 1, value: 0, isRising: false, isFalling: true);
      mpc.onEdge(falling);
      expect([mpc.getPhase(0), mpc.getPhase(1), mpc.getPhase(2)], [0, 0, 0]);
    });

    test('out-of-range phase index throws', () {
      final mpc = MultiPhaseClock(2);
      expect(() => mpc.getPhase(2), throwsRangeError);
      expect(() => mpc.getPhase(-1), throwsRangeError);
    });

    test('validation precedes allocation', () {
      expect(
          () => MultiPhaseClock(0),
          throwsA(isA<ArgumentError>().having(
              (e) => e.message, 'message', contains('Phases must be >= 2'))));
      expect(() => MultiPhaseClock(-3), throwsArgumentError);
    });
  });

  group('EXTRA: edges and errors', () {
    test('edges compare by value', () {
      const a = ClockEdge(cycle: 1, value: 1, isRising: true, isFalling: false);
      const b = ClockEdge(cycle: 1, value: 1, isRising: true, isFalling: false);
      const c = ClockEdge(cycle: 1, value: 0, isRising: false, isFalling: true);
      expect(a, equals(b));
      expect(a.hashCode, b.hashCode);
      expect(a == c, isFalse);
      expect(a == 'edge', isFalse);
      expect(a.toString(), contains('cycle: 1'));
    });

    test('validation messages are exact', () {
      expect(
          () => Clock(0),
          throwsA(isA<ArgumentError>().having(
              (e) => e.message, 'message', 'frequency_hz must be > 0')));
      expect(() => Clock(-5), throwsArgumentError);
      expect(
          () => ClockDivider(100, 0),
          throwsA(isA<ArgumentError>().having(
              (e) => e.message, 'message', 'Divisor must be >= 2, got 0')));
    });

    test('run of zero cycles returns no edges', () {
      final clk = Clock(100);
      expect(clk.run(0), isEmpty);
      expect(clk.totalTicks, 0);
    });
  });
}
