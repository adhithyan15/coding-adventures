import 'dart:math';

import 'package:coding_adventures_gradient_descent/gradient_descent.dart';
import 'package:test/test.dart';

/// Matcher for an [InvalidGradientDescentInputError] with [code].
Matcher throwsCode(GradientDescentErrorCode code) => throwsA(
      isA<InvalidGradientDescentInputError>()
          .having((e) => e.errorCode, 'errorCode', code)
          .having((e) => e.errorId, 'errorId', code.errorId),
    );

void main() {
  group('ML02 spec vectors', () {
    test('parity vector matches the spec within 1e-12 and bit-exactly', () {
      final weights = [1.0, -0.5, 2.0];
      final gradients = [0.1, -0.2, 0.0];
      const lr = 0.1;
      final result = sgd(weights, gradients, lr);
      final expected = [0.99, -0.48, 2.0];
      expect(result, hasLength(3));
      for (var i = 0; i < 3; i++) {
        expect(result[i], closeTo(expected[i], 1e-12));
        // Unfused reference: multiply, round, subtract, round.
        final step = lr * gradients[i];
        expect(result[i], equals(weights[i] - step));
      }
    });

    test('dyadic witness is exact', () {
      expect(
        sgd([1.0, -0.5, 2.0], [0.5, -0.25, 0.0], 0.5),
        equals([0.75, -0.375, 2.0]),
      );
    });

    test('result is unmodifiable and inputs are not mutated', () {
      final weights = [1.0, 2.0];
      final gradients = [0.5, 0.25];
      final result = sgd(weights, gradients, 0.5);
      expect(() => result[0] = 9.0, throwsUnsupportedError);
      expect(weights, equals([1.0, 2.0]));
      expect(gradients, equals([0.5, 0.25]));
    });

    test('package metadata is exported', () {
      expect(gradientDescentVersion, equals('0.1.0'));
      expect(maxVectorLength, equals(1048576));
    });
  });

  group('5-Class ML02 Optimizer Security Resilience Showcase', () {
    test('Class 1: validation precedence is deterministic', () {
      // length-mismatch beats empty-input
      expect(() => sgd([], [1.0], 0.1),
          throwsCode(GradientDescentErrorCode.lengthMismatch));
      // empty-input beats non-finite learning rate
      expect(() => sgd([], [], double.nan),
          throwsCode(GradientDescentErrorCode.emptyInput));
      // input-too-large beats non-finite learning rate
      final big = List<double>.filled(maxVectorLength + 1, 0.0);
      expect(() => sgd(big, big, double.nan),
          throwsCode(GradientDescentErrorCode.inputTooLarge));
      // non-finite learning rate beats non-finite input
      expect(() => sgd([double.nan], [1.0], double.infinity),
          throwsCode(GradientDescentErrorCode.nonFiniteLearningRate));
      // non-finite input (anywhere) beats overflow (earlier element)
      expect(() => sgd([1e308, 1.0], [-1e308, double.nan], 1.0),
          throwsCode(GradientDescentErrorCode.nonFiniteInput));
    });

    test('Class 2: CWE-400 pre-allocation ceiling at the exact boundary', () {
      final atLimit = List<double>.filled(maxVectorLength, 1.0);
      final result = sgd(atLimit, atLimit, 0.5);
      expect(result, hasLength(maxVectorLength));
      expect(result.first, equals(0.5));
      final overLimit = List<double>.filled(maxVectorLength + 1, 1.0);
      expect(() => sgd(overLimit, overLimit, 0.5),
          throwsCode(GradientDescentErrorCode.inputTooLarge));
    });

    test('Class 3: CWE-682 NaN/Infinity poisoning and overflow refusal', () {
      for (final bad in [
        double.nan,
        double.infinity,
        double.negativeInfinity
      ]) {
        expect(() => sgd([1.0], [1.0], bad),
            throwsCode(GradientDescentErrorCode.nonFiniteLearningRate));
        expect(() => sgd([bad], [1.0], 0.1),
            throwsCode(GradientDescentErrorCode.nonFiniteInput));
        expect(() => sgd([1.0], [bad], 0.1),
            throwsCode(GradientDescentErrorCode.nonFiniteInput));
      }
      expect(() => sgd([1e308], [-1e308], 1.0),
          throwsCode(GradientDescentErrorCode.numericOverflow));
      // Overflow inside the multiply step is refused as well.
      expect(() => sgd([0.0], [1e200], 1e200),
          throwsCode(GradientDescentErrorCode.numericOverflow));
    });

    test('Class 4: aliasing and caller-mutation isolation', () {
      final x = [1.0, -2.0, 4.0];
      final result = sgd(x, x, 0.5);
      expect(result, equals([0.5, -1.0, 2.0]));
      expect(x, equals([1.0, -2.0, 4.0]));
      x[0] = 100.0;
      expect(result[0], equals(0.5));
    });

    test('Class 5: CWE-209 payload-blind diagnostics', () {
      const sentinelWeight = 13371337.0;
      const sentinelRate = 0.4242;
      final weights = List<double>.filled(7, sentinelWeight);
      final gradients = List<double>.filled(7, double.nan);
      final forbidden = [
        sentinelWeight.toString(),
        '13371337',
        sentinelRate.toString(),
        '7',
      ];
      final failures = <Object>[];
      for (final run in <void Function()>[
        () => sgd(weights, gradients, sentinelRate),
        () => sgd(weights, [1.0], sentinelRate),
        () => sgd([sentinelWeight], [1.0], double.nan),
      ]) {
        try {
          run();
        } on InvalidGradientDescentInputError catch (e) {
          failures.add(e);
          for (final text in [e.message.toString(), e.toString()]) {
            for (final secret in forbidden) {
              expect(text.contains(secret), isFalse,
                  reason: 'diagnostic leaked caller data');
            }
          }
          expect(e.invalidValue, isNull);
          expect(e.name, isNull);
        }
      }
      expect(failures, hasLength(3));
    });

    test('every error code has a stable id and static message', () {
      final ids = GradientDescentErrorCode.values.map((c) => c.errorId).toSet();
      expect(ids, hasLength(GradientDescentErrorCode.values.length));
      expect(GradientDescentErrorCode.nullInput.errorId, equals('null-input'));
      for (final code in GradientDescentErrorCode.values) {
        final error = InvalidGradientDescentInputError(code);
        expect(error.message, equals(code.message));
        expect(error, isA<GradientDescentError>());
        expect(error, isA<ArgumentError>());
      }
    });
  });

  group('PRNG dyadic multi-step linearity property', () {
    test('two steps with a then b equal one step with a + b, exactly', () {
      final rng = Random(0xC0FFEE);
      // Dyadic values k / 2^m with small k keep every product and sum exact
      // in binary64, so the identity must hold bit-for-bit.
      double dyadic(int maxK, int shift) =>
          (rng.nextInt(2 * maxK + 1) - maxK) / (1 << shift);
      for (var iteration = 0; iteration < 500; iteration++) {
        final length = 1 + rng.nextInt(16);
        final weights = List<double>.generate(length, (_) => dyadic(64, 4));
        final gradients = List<double>.generate(length, (_) => dyadic(64, 4));
        final a = dyadic(8, 3);
        final b = dyadic(8, 3);
        final twoSteps = sgd(sgd(weights, gradients, a), gradients, b);
        final oneStep = sgd(weights, gradients, a + b);
        expect(twoSteps, equals(oneStep));
      }
    });
  });

  group('10,000-iteration adversarial mutation fuzz loop', () {
    test('either all-finite output or a known library error', () {
      final rng = Random(0x5EED1234);
      const specials = [
        double.nan,
        double.infinity,
        double.negativeInfinity,
        0.0,
        -0.0,
        1e308,
        -1e308,
        4.9e-324,
        double.maxFinite,
        double.minPositive,
      ];
      double pick() => rng.nextInt(4) == 0
          ? specials[rng.nextInt(specials.length)]
          : (rng.nextDouble() - 0.5) * 1e6;
      final known = GradientDescentErrorCode.values.toSet();
      var accepted = 0;
      var rejected = 0;
      for (var iteration = 0; iteration < 10000; iteration++) {
        final wLen = rng.nextInt(9);
        final gLen = rng.nextInt(10) == 0 ? rng.nextInt(9) : wLen;
        final weights = List<double>.generate(wLen, (_) => pick());
        final gradients = List<double>.generate(gLen, (_) => pick());
        final lr = pick();
        try {
          final result = sgd(weights, gradients, lr);
          expect(result, hasLength(wLen));
          expect(result.every((v) => v.isFinite), isTrue);
          accepted++;
        } on InvalidGradientDescentInputError catch (e) {
          expect(known.contains(e.errorCode), isTrue);
          rejected++;
        }
      }
      expect(accepted, greaterThan(0));
      expect(rejected, greaterThan(0));
    });
  });
}
