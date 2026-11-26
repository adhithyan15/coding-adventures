/// Property tests for `coding_adventures_clock`, using the in-lane `pbt`
/// package: tick/edge/run/reset laws, divider ratios, multiphase rotation,
/// listener delivery, and the period formula — fixed seeds.
library;

import 'package:coding_adventures_clock/coding_adventures_clock.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('tick law: value, cycle, and count follow n', () {
      final gen = Gen.integer(BigInt.zero, BigInt.from(500));
      final verdict = checkProperty<BigInt>(
        BigInt.from(81),
        PbtOptions(numTests: 200, seed: BigInt.from(81)),
        (rng) => gen.sample(rng),
        (n) {
          final ticks = n.toInt();
          final clk = Clock(100);
          for (var i = 0; i < ticks; i++) {
            clk.tick();
          }
          return clk.value == ticks % 2 &&
              clk.cycle == (ticks + 1) ~/ 2 &&
              clk.totalTicks == ticks;
        },
      );
      _check('tick-law', verdict);
    });

    test('edge stream alternates rising-first with matching cycles', () {
      final gen = Gen.integer(BigInt.zero, BigInt.from(200));
      final verdict = checkProperty<BigInt>(
        BigInt.from(82),
        PbtOptions(numTests: 200, seed: BigInt.from(82)),
        (rng) => gen.sample(rng),
        (n) {
          final ticks = n.toInt();
          final clk = Clock(100);
          for (var i = 0; i < ticks; i++) {
            final edge = clk.tick();
            final wantRising = i.isEven;
            if (edge.isRising != wantRising ||
                edge.isFalling == wantRising ||
                edge.value != (wantRising ? 1 : 0) ||
                edge.cycle != i ~/ 2 + 1) {
              return false;
            }
          }
          return true;
        },
      );
      _check('edge-stream', verdict);
    });

    test('run(n) emits 2n edges and reset restores rest', () {
      final gen = Gen.tuple2(
        Gen.integer(BigInt.zero, BigInt.from(100)),
        Gen.integer(BigInt.zero, BigInt.from(100)),
      );
      final verdict = checkProperty<(BigInt, BigInt)>(
        BigInt.from(83),
        PbtOptions(numTests: 200, seed: BigInt.from(83)),
        (rng) => gen.sample(rng),
        (input) {
          final a = input.$1.toInt();
          final b = input.$2.toInt();
          final clk = Clock(100);
          final edges = clk.run(a);
          if (edges.length != 2 * a || clk.cycle != a) return false;
          clk.reset();
          if (clk.value != 0 || clk.cycle != 0 || clk.totalTicks != 0) {
            return false;
          }
          clk.run(b);
          return clk.cycle == b && clk.totalTicks == 2 * b;
        },
      );
      _check('run-reset', verdict);
    });

    test('divider ratio: output cycles equal rising ~/ divisor', () {
      final gen = Gen.tuple2(
        Gen.integer(BigInt.from(2), BigInt.from(8)),
        Gen.integer(BigInt.zero, BigInt.from(300)),
      );
      final verdict = checkProperty<(BigInt, BigInt)>(
        BigInt.from(84),
        PbtOptions(numTests: 200, seed: BigInt.from(84)),
        (rng) => gen.sample(rng),
        (input) {
          final divisor = input.$1.toInt();
          final ticks = input.$2.toInt();
          final source = Clock(1000);
          final divider = ClockDivider(1000, divisor);
          for (var i = 0; i < ticks; i++) {
            divider.onEdge(source.tick());
          }
          final rising = (ticks + 1) ~/ 2;
          return divider.output.cycle == rising ~/ divisor &&
              divider.output.totalTicks == 2 * (rising ~/ divisor) &&
              divider.output.value == 0;
        },
      );
      _check('divider-ratio', verdict);
    });

    test('divider output frequency is the integer quotient', () {
      final gen = Gen.tuple2(
        Gen.integer(BigInt.one, BigInt.from(1000000000)),
        Gen.integer(BigInt.from(2), BigInt.from(16)),
      );
      final verdict = checkProperty<(BigInt, BigInt)>(
        BigInt.from(85),
        PbtOptions(numTests: 200, seed: BigInt.from(85)),
        (rng) => gen.sample(rng),
        (input) {
          final freq = input.$1.toInt();
          final divisor = input.$2.toInt();
          // Quotients of zero fail construction (frequency must be > 0),
          // exactly as in the reference.
          if (freq ~/ divisor == 0) {
            try {
              ClockDivider(freq, divisor);
              return false;
            } on ArgumentError {
              return true;
            }
          }
          return ClockDivider(freq, divisor).output.frequencyHz ==
              freq ~/ divisor;
        },
      );
      _check('divider-frequency', verdict);
    });

    test('multiphase rotation: one hot phase at (R-1) % P', () {
      final gen = Gen.tuple2(
        Gen.integer(BigInt.from(2), BigInt.from(8)),
        Gen.integer(BigInt.zero, BigInt.from(100)),
      );
      final verdict = checkProperty<(BigInt, BigInt)>(
        BigInt.from(86),
        PbtOptions(numTests: 200, seed: BigInt.from(86)),
        (rng) => gen.sample(rng),
        (input) {
          final phases = input.$1.toInt();
          final ticks = input.$2.toInt();
          final source = Clock(100);
          final mpc = MultiPhaseClock(phases);
          for (var i = 0; i < ticks; i++) {
            mpc.onEdge(source.tick());
          }
          final rising = (ticks + 1) ~/ 2;
          for (var p = 0; p < phases; p++) {
            final want = (rising > 0 && p == (rising - 1) % phases) ? 1 : 0;
            if (mpc.getPhase(p) != want) return false;
          }
          return true;
        },
      );
      _check('multiphase-rotation', verdict);
    });

    test('listeners observe every tick in order', () {
      final gen = Gen.tuple2(
        Gen.integer(BigInt.from(1), BigInt.from(4)),
        Gen.integer(BigInt.zero, BigInt.from(100)),
      );
      final verdict = checkProperty<(BigInt, BigInt)>(
        BigInt.from(87),
        PbtOptions(numTests: 200, seed: BigInt.from(87)),
        (rng) => gen.sample(rng),
        (input) {
          final count = input.$1.toInt();
          final ticks = input.$2.toInt();
          final clk = Clock(100);
          final seen = List<int>.filled(count, 0);
          for (var k = 0; k < count; k++) {
            clk.registerListener((_) => seen[k]++);
          }
          for (var i = 0; i < ticks; i++) {
            clk.tick();
          }
          return seen.every((n) => n == ticks);
        },
      );
      _check('listener-delivery', verdict);
    });

    test('period inverts frequency within fp tolerance', () {
      final gen = Gen.integer(BigInt.one, BigInt.from(1000000000000));
      final verdict = checkProperty<BigInt>(
        BigInt.from(88),
        PbtOptions(numTests: 200, seed: BigInt.from(88)),
        (rng) => gen.sample(rng),
        (freq) {
          final f = freq.toInt();
          final rel = ((Clock(f).periodNs() * f - 1e9).abs()) / 1e9;
          return rel < 1e-12;
        },
      );
      _check('period-formula', verdict);
    });
  });
}
