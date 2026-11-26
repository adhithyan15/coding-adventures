/// Bounded fuzz campaign for `coding_adventures_perceptron`, using the
/// in-lane `fuzz` package: hostile datasets (empty, ragged, mismatched,
/// NaN/Inf/extreme values) go through fit and predict. Only the documented
/// errors ([ArgumentError], [RangeError], [UnsupportedError]) may escape;
/// finite well-formed inputs must yield finite in-range predictions with
/// exact shapes. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_perceptron/coding_adventures_perceptron.dart';
import 'package:test/test.dart';

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

  /// A hostile double: small ints dominate, with extremes and specials.
  double takeDouble() {
    switch (takeByte() % 16) {
      case 0:
        return double.nan;
      case 1:
        return double.infinity;
      case 2:
        return double.negativeInfinity;
      case 3:
        return 1e308;
      case 4:
        return -1e308;
      default:
        return ((takeByte() % 21) - 10).toDouble();
    }
  }

  List<List<double>> takeMatrix() {
    final rows = takeByte() % 5;
    return [
      for (var i = 0; i < rows; i++)
        [for (var j = 0, n = takeByte() % 5; j < n; j++) takeDouble()]
    ];
  }
}

/// Finite values small enough that ten epochs cannot overflow to
/// infinity (|v| <= 100 keeps every intermediate below ~1e6).
bool _smallMatrix(List<List<double>> m) =>
    m.every((row) => row.every((v) => v.isFinite && v.abs() <= 100));

/// Non-empty, rectangular, non-empty rows: exactly what [Matrix] accepts.
bool _matrixOk(List<List<double>> m) {
  if (m.isEmpty) return false;
  final width = m[0].length;
  if (width == 0) return false;
  return m.every((row) => row.length == width);
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final x = d.takeMatrix();
    final y = d.takeMatrix();
    final lr = (d.takeByte() % 21 - 10).toDouble() / 10.0;
    final epochs = d.takeByte() % 11;
    final logSteps = d.takeByte() % 4;
    cov.recordEdge(x.length);
    cov.recordEdge(10 + y.length);

    // Outcome model, mirroring fit's failure order exactly:
    // Matrix construction (ArgumentError) -> label overrun (RangeError,
    // only when x outgrows y) -> log modulo (UnsupportedError at zero).
    // Extra y rows/columns are silently ignored, as in the reference.
    final shapesOk = _matrixOk(x) && _matrixOk(y);
    final overrun = shapesOk && x.length > y.length;
    final accepted = shapesOk && !overrun;
    final finiteInputs =
        accepted && _smallMatrix(x) && _smallMatrix(y) && lr.isFinite;

    final p = Perceptron(lr, epochs);
    try {
      var logs = 0;
      p.fit(x, y, logSteps, log: (_) => logs++);
      if (!accepted) {
        return const OutcomeCrash(
            'missing-rejection', ['fit-accepted-hostile']);
      }
      if (finiteInputs) {
        // Finite accepted inputs: finite in-range predictions, exact
        // weight shape.
        final preds = p.predict(x);
        if (preds.length != x.length) {
          return const OutcomeCrash('shape-divergence', ['prediction-count']);
        }
        for (final v in preds) {
          if (v.isNaN || v < 0.0 || v > 1.0) {
            return const OutcomeCrash('range-divergence', ['prediction-range']);
          }
        }
        if (p.weights!.rows != x[0].length) {
          return const OutcomeCrash('shape-divergence', ['weight-shape']);
        }
      }
      cov.recordEdge(20 + logs.clamp(0, 5));
    } on RangeError {
      // Most specific first: RangeError subtypes ArgumentError.
      if (!(shapesOk && overrun)) {
        return const OutcomeCrash('spurious-rejection', ['unexpected-range']);
      }
      cov.recordEdge(32);
    } on ArgumentError {
      if (shapesOk) {
        return const OutcomeCrash('spurious-rejection', ['fit-rejected-valid']);
      }
      cov.recordEdge(30);
    } on UnsupportedError {
      if (!(shapesOk && !overrun && logSteps == 0)) {
        return const OutcomeCrash(
            'spurious-rejection', ['unexpected-unsupported']);
      }
      cov.recordEdge(31);
    }
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([2, 2, 0, 1, 1, 0, 2, 0, 1, 5, 3, 1]),
      Uint8List.fromList([4, 1, 1, 1, 1, 0, 0, 0, 0, 10, 10, 2]),
      Uint8List.fromList([0, 3, 255, 255, 255, 1, 0, 0]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 151);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(151)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: contracts hold, no crashes', () {
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
