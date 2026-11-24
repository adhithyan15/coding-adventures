/// Unit tests for `coding_adventures_perceptron`.
///
/// The Rust reference ships no tests for this crate, so these are designed
/// from its contract: constructor state, AND/OR learning to perfection,
/// loss decrease, determinism, refit reset, log format and count, shape
/// and domain rejection, and unfitted prediction.
library;

import 'package:coding_adventures_loss_functions/loss_functions.dart';
import 'package:coding_adventures_perceptron/coding_adventures_perceptron.dart';
import 'package:test/test.dart';

const _andX = [
  [0.0, 0.0],
  [0.0, 1.0],
  [1.0, 0.0],
  [1.0, 1.0],
];
const _andY = [
  [0.0],
  [0.0],
  [0.0],
  [1.0],
];
const _orY = [
  [0.0],
  [1.0],
  [1.0],
  [1.0],
];

List<int> _rounded(List<double> probs) =>
    [for (final p in probs) p >= 0.5 ? 1 : 0];

void main() {
  group('construction', () {
    test('starts unfitted with given hyperparameters', () {
      final p = Perceptron(0.5, 100);
      expect(p.learningRate, 0.5);
      expect(p.epochs, 100);
      expect(p.weights, isNull);
      expect(p.bias, 0.0);
    });

    test('predict before fit throws StateError', () {
      expect(() => Perceptron(0.5, 10).predict(_andX), throwsStateError);
    });
  });

  group('learning', () {
    test('AND gate classifies perfectly', () {
      final p = Perceptron(1.0, 200);
      p.fit(_andX, _andY, 1000000);
      expect(_rounded(p.predict(_andX)), [0, 0, 0, 1]);
      // Wide margins, not a near-miss.
      final probs = p.predict(_andX);
      expect(probs[1], lessThan(0.2));
      expect(probs[3], greaterThan(0.8));
    });

    test('OR gate classifies perfectly', () {
      final p = Perceptron(1.0, 200);
      p.fit(_andX, _orY, 1000000);
      expect(_rounded(p.predict(_andX)), [0, 1, 1, 1]);
    });

    test('loss decreases on AND', () {
      final before = bce([0.0, 0.0, 0.0, 1.0], [0.5, 0.5, 0.5, 0.5]);
      final p = Perceptron(1.0, 200);
      p.fit(_andX, _andY, 1000000);
      final after = bce([0.0, 0.0, 0.0, 1.0], p.predict(_andX));
      expect(after, lessThan(before / 4));
    });

    test('weights take the feature shape and bias moves', () {
      final p = Perceptron(0.5, 10);
      p.fit(_andX, _andY, 1000000);
      expect(p.weights!.rows, 2);
      expect(p.weights!.data[0].length, 1);
      expect(p.bias, isNot(0.0));
    });

    test('predictions stay strictly inside (0, 1)', () {
      final p = Perceptron(1.0, 200);
      p.fit(_andX, _andY, 1000000);
      for (final prob in p.predict(_andX)) {
        expect(prob, greaterThan(0.0));
        expect(prob, lessThan(1.0));
      }
    });

    test('fitting is deterministic', () {
      Perceptron trained() {
        final p = Perceptron(0.7, 50);
        p.fit(_andX, _andY, 1000000);
        return p;
      }

      final first = trained();
      final second = trained();
      expect(second.weights!.data, first.weights!.data);
      expect(second.bias, first.bias);
    });

    test('refitting restarts from zero', () {
      final p = Perceptron(1.0, 200);
      p.fit(_andX, _andY, 1000000);
      final andBias = p.bias;
      p.fit(_andX, _orY, 1000000);
      expect(p.bias, isNot(andBias));
      expect(_rounded(p.predict(_andX)), [0, 1, 1, 1]);
    });
  });

  group('logging', () {
    test('log lines match the reference format', () {
      final lines = <String>[];
      final p = Perceptron(0.5, 6);
      p.fit(_andX, _andY, 3, log: lines.add);
      expect(lines.length, 3); // epochs 0, 3, 6
      expect(
          lines[0],
          matches(RegExp(
              r'^Epoch +\d+ \| BCE Loss: \d+\.\d{4} \| Bias: -?\d+\.\d{2}$')));
      expect(lines[0].startsWith('Epoch    0 |'), isTrue);
    });

    test('silent by default', () {
      // No log callback: fit completes (nothing to observe stdout here,
      // so this pins that logging is optional, not the silence itself).
      final p = Perceptron(0.5, 5);
      p.fit(_andX, _andY, 1);
      expect(p.weights, isNotNull);
    });

    test('zero logSteps throws', () {
      final p = Perceptron(0.5, 5);
      expect(() => p.fit(_andX, _andY, 0), throwsA(isA<UnsupportedError>()));
    });
  });

  group('rejection', () {
    test('empty inputs throw ArgumentError', () {
      final p = Perceptron(0.5, 5);
      expect(() => p.fit([], [], 1), throwsArgumentError);
    });

    test('mismatched row counts throw RangeError', () {
      // y runs out mid-loop, as the reference index-panics mid-loop.
      final p = Perceptron(0.5, 5);
      expect(
          () => p.fit(
              _andX,
              [
                [0.0]
              ],
              1),
          throwsRangeError);
    });

    test('extra label rows are silently ignored', () {
      // x runs out first, so the loop never reaches y's tail — same in
      // the reference (it indexes y by the feature loop).
      final p = Perceptron(0.5, 5);
      p.fit([
        [0.0, 0.0]
      ], [
        [0.0],
        [1.0],
        [1.0]
      ], 1000000);
      expect(
          p.predict([
            [0.0, 0.0]
          ]).length,
          1);
    });

    test('ragged rows throw ArgumentError', () {
      final p = Perceptron(0.5, 5);
      expect(
          () => p.fit([
                [0.0, 0.0],
                [1.0]
              ], _andY, 1),
          throwsArgumentError);
    });

    test('non-bit labels still compute (documented lane behavior)', () {
      // Labels are continuous targets for BCE (0.0-1.0); the gates only
      // ever see features-derived values. Soft labels must not throw.
      final p = Perceptron(0.5, 5);
      p.fit(
          _andX,
          [
            [0.0],
            [0.25],
            [0.75],
            [1.0]
          ],
          1000000);
      expect(p.predict(_andX).length, 4);
    });
  });
}
