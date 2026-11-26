/// Property tests for `coding_adventures_perceptron`, using the in-lane
/// `pbt` package: shapes, determinism, probability ranges, loss decrease
/// on separable data, and refit reset — fixed seeds.
library;

import 'package:coding_adventures_loss_functions/loss_functions.dart';
import 'package:coding_adventures_perceptron/coding_adventures_perceptron.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// A random dataset: 1-6 rows, 1-4 features, binary labels.
Gen<(List<List<double>>, List<List<double>>)> _dataset() => Gen.tuple2(
            Gen.integer(BigInt.one, BigInt.from(6)),
            Gen.integer(BigInt.one, BigInt.from(4)))
        .flatMap((shape) {
      final rows = shape.$1.toInt();
      final features = shape.$2.toInt();
      return Gen.tuple2(
        Gen.list(
            rows,
            rows,
            Gen.list(features, features,
                Gen.integer(BigInt.zero, BigInt.one).map((b) => b.toDouble()))),
        Gen.list(rows, rows,
            Gen.integer(BigInt.zero, BigInt.one).map((b) => [b.toDouble()])),
      );
    });

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('shapes: weights, predictions, and bias', () {
      final verdict = checkProperty<(List<List<double>>, List<List<double>>)>(
        BigInt.from(141),
        PbtOptions(numTests: 200, seed: BigInt.from(141)),
        (rng) => _dataset().sample(rng),
        (data) {
          final x = data.$1;
          final y = data.$2;
          final p = Perceptron(0.5, 5);
          p.fit(x, y, 1000000);
          final preds = p.predict(x);
          return p.weights!.rows == x[0].length &&
              p.weights!.data[0].length == 1 &&
              preds.length == x.length &&
              p.bias.isFinite &&
              p.weights!.data.every((row) => row.every((v) => v.isFinite));
        },
      );
      _check('shapes', verdict);
    });

    test('fitting is deterministic', () {
      final verdict = checkProperty<(List<List<double>>, List<List<double>>)>(
        BigInt.from(142),
        PbtOptions(numTests: 200, seed: BigInt.from(142)),
        (rng) => _dataset().sample(rng),
        (data) {
          Perceptron trained() {
            final p = Perceptron(0.7, 20);
            p.fit(data.$1, data.$2, 1000000);
            return p;
          }

          final first = trained();
          final second = trained();
          return second.weights!.data.toString() ==
                  first.weights!.data.toString() &&
              second.bias == first.bias;
        },
      );
      _check('determinism', verdict);
    });

    test('predictions are probabilities', () {
      final verdict = checkProperty<(List<List<double>>, List<List<double>>)>(
        BigInt.from(143),
        PbtOptions(numTests: 200, seed: BigInt.from(143)),
        (rng) => _dataset().sample(rng),
        (data) {
          final p = Perceptron(0.5, 10);
          p.fit(data.$1, data.$2, 1000000);
          return p.predict(data.$1).every((v) => v >= 0.0 && v <= 1.0);
        },
      );
      _check('probability-range', verdict);
    });

    test('loss decreases on the AND gate', () {
      final verdict = checkProperty<int>(
        BigInt.from(144),
        PbtOptions(numTests: 50, seed: BigInt.from(144)),
        (rng) => Gen.u8().sample(rng) % 3,
        (variant) {
          // Three learning rates; all must improve over the 0.5 baseline.
          const x = [
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 0.0],
            [1.0, 1.0]
          ];
          const truth = [0.0, 0.0, 0.0, 1.0];
          final lr = [0.1, 0.5, 1.0][variant];
          final p = Perceptron(lr, 100);
          p.fit(x, truth.map((t) => [t]).toList(), 1000000);
          final after = bce(truth, p.predict(x));
          return after < 0.6931471805599453;
        },
      );
      _check('loss-decreases', verdict);
    });

    test('refit restarts from zero weights', () {
      final verdict = checkProperty<(List<List<double>>, List<List<double>>)>(
        BigInt.from(145),
        PbtOptions(numTests: 200, seed: BigInt.from(145)),
        (rng) => _dataset().sample(rng),
        (data) {
          final p = Perceptron(0.5, 10);
          p.fit(data.$1, data.$2, 1000000);
          final once = p.weights!.data.map((row) => row.toList()).toList();
          final biasOnce = p.bias;
          p.fit(data.$1, data.$2, 1000000);
          return p.weights!.data.toString() == once.toString() &&
              p.bias == biasOnce;
        },
      );
      _check('refit-reset', verdict);
    });
  });
}
