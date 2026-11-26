/// Script-driven differential fuzz campaign for `coding_adventures_clock`,
/// using the in-lane `fuzz` package: hostile byte strings decode into
/// operation scripts (ticks, cycles, runs, resets, synthetic edges) over
/// one source clock feeding a divider and a multiphase clock, and a shadow
/// model predicts every observable after every op. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_clock/coding_adventures_clock.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

/// Shadow model: plain counters predicting the three clocks.
class _Model {
  int srcTicks = 0;

  /// Ticks ever observed by the listener. Unlike [srcTicks], this never
  /// resets: listeners survive [Clock.reset], exactly as in the library.
  int heard = 0;
  int divRising = 0;
  int mpcRising = 0;
  final int divisor;
  final int phases;

  _Model(this.divisor, this.phases);

  int get srcValue => srcTicks % 2;
  int get srcCycle => (srcTicks + 1) ~/ 2;
  int get srcRising => (srcTicks + 1) ~/ 2;

  int get divOutCycles => divRising ~/ divisor;
  int get divOutTicks => 2 * (divRising ~/ divisor);

  int mpcPhase(int p) =>
      (mpcRising > 0 && p == (mpcRising - 1) % phases) ? 1 : 0;
}

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield 0), so the decoder itself can never throw.
class _Decoder {
  final Uint8List input;
  int cursor = 0;

  _Decoder(this.input);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }
}

const _syntheticRising =
    ClockEdge(cycle: 999, value: 1, isRising: true, isFalling: false);
const _syntheticFalling =
    ClockEdge(cycle: 999, value: 0, isRising: false, isFalling: true);

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final divisor = d.takeByte() % 7 + 2;
    final phases = d.takeByte() % 7 + 2;
    final source = Clock(1000);
    final divider = ClockDivider(1000, divisor);
    final mpc = MultiPhaseClock(phases);
    final model = _Model(divisor, phases);
    var listenerCalls = 0;
    source.registerListener((_) => listenerCalls++);

    bool check() {
      if (source.value != model.srcValue) return false;
      if (source.cycle != model.srcCycle) return false;
      if (source.totalTicks != model.srcTicks) return false;
      if (listenerCalls != model.heard) return false;
      if (divider.output.cycle != model.divOutCycles) return false;
      if (divider.output.totalTicks != model.divOutTicks) return false;
      if (divider.output.value != 0) return false;
      for (var p = 0; p < phases; p++) {
        final v = mpc.getPhase(p);
        if (v != 0 && v != 1) return false;
        if (v != model.mpcPhase(p)) return false;
      }
      return true;
    }

    final ops = d.takeByte() % 25 + 1;
    for (var i = 0; i < ops; i++) {
      final op = d.takeByte() % 7;
      cov.recordEdge(op);
      switch (op) {
        case 0:
          divider.onEdge(source.tick());
          mpc.onEdge(_syntheticFalling); // ignored no-op probe
          model.srcTicks++;
          model.heard++;
          // The divider saw the real tick; count its rising half.
          if (model.srcTicks.isOdd) model.divRising++;
        // The mpc saw only a synthetic falling edge: unchanged.
        case 1:
          source.fullCycle();
          model.srcTicks += 2;
          model.heard += 2;
        case 2:
          final k = d.takeByte() % 9;
          source.run(k);
          model.srcTicks += 2 * k;
          model.heard += 2 * k;
        case 3:
          source.reset();
          model.srcTicks = 0;
        // Divider, multiphase, and listener counts have no reset: their
        // shadow counters persist, exactly as in the library.
        case 4:
          divider.onEdge(_syntheticRising);
          mpc.onEdge(_syntheticRising);
          model.divRising++;
          model.mpcRising++;
        case 5:
          divider.onEdge(_syntheticFalling);
          mpc.onEdge(_syntheticFalling);
        // Falling edges change nothing on either side.
        case 6:
          final edge = source.tick();
          divider.onEdge(edge);
          mpc.onEdge(edge);
          model.srcTicks++;
          model.heard++;
          if (model.srcTicks.isOdd) {
            model.divRising++;
            model.mpcRising++;
          }
      }
      if (!check()) {
        return const OutcomeCrash('model-divergence', ['state-mismatch']);
      }
    }
    cov.recordEdge(10 + (model.srcTicks % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([4, 4, 10, 0, 0, 0, 0, 0, 0, 0, 0]),
      Uint8List.fromList([2, 2, 8, 6, 6, 6, 6, 4, 5, 3, 1]),
      Uint8List.fromList([8, 8, 20, 2, 5, 0, 1, 4, 4, 2, 7]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 89); longer
/// ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(89)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: model agrees, no crashes', () {
      final report = runCampaign();
      expect(report.crashes, isEmpty,
          reason: report.crashes
              .map((c) =>
                  '${c.signature} ${c.kind} ${c.frames} input=${c.minimalInput}')
              .join('; '));
      expect(report.iterations, greaterThan(100));
      expect(report.totalEdges, greaterThan(5));
    });
  });
}
