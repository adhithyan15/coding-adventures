import 'dart:math' as math;

import 'package:coding_adventures_activation_functions/coding_adventures_activation_functions.dart';
import 'package:test/test.dart';

// Reference values taken from the Rust activation-functions crate's tests.
const tol = 1e-12;

void main() {
  group('linear', () {
    test('is the identity', () {
      expect(linear(-3.0), closeTo(-3.0, tol));
      expect(linear(0.0), closeTo(0.0, tol));
      expect(linear(5.0), closeTo(5.0, tol));
    });
    test('derivative is always 1', () {
      for (final x in [-3.0, 0.0, 5.0]) {
        expect(linearDerivative(x), closeTo(1.0, tol));
      }
    });
  });

  group('sigmoid', () {
    test('matches reference values', () {
      expect(sigmoid(0.0), closeTo(0.5, tol));
      expect(sigmoid(1.0), closeTo(0.7310585786300049, tol));
      expect(sigmoid(-1.0), closeTo(0.2689414213699951, tol));
      expect(sigmoid(10.0), closeTo(0.9999546021312976, tol));
    });
    test('saturates and never overflows at extremes', () {
      expect(sigmoid(-710.0), closeTo(0.0, tol));
      expect(sigmoid(710.0), closeTo(1.0, tol));
      expect(sigmoid(1e9), closeTo(1.0, tol));
      expect(sigmoid(-1e9), closeTo(0.0, tol));
    });
    test('derivative matches reference values', () {
      expect(sigmoidDerivative(0.0), closeTo(0.25, tol));
      expect(sigmoidDerivative(1.0), closeTo(0.19661193324148185, tol));
    });
  });

  group('relu', () {
    test('is piecewise max(0, x)', () {
      expect(relu(5.0), closeTo(5.0, tol));
      expect(relu(-3.0), closeTo(0.0, tol));
      expect(relu(0.0), closeTo(0.0, tol));
    });
    test('derivative is 1 for positive, 0 otherwise (0 at 0)', () {
      expect(reluDerivative(5.0), closeTo(1.0, tol));
      expect(reluDerivative(-3.0), closeTo(0.0, tol));
      expect(reluDerivative(0.0), closeTo(0.0, tol));
    });
  });

  group('leakyRelu', () {
    test('keeps a small negative slope', () {
      expect(leakyRelu(5.0), closeTo(5.0, tol));
      expect(leakyRelu(-3.0), closeTo(-0.03, tol));
      expect(leakyRelu(0.0), closeTo(0.0, tol));
    });
    test('derivative is 1 for positive, slope otherwise', () {
      expect(leakyReluDerivative(5.0), closeTo(1.0, tol));
      expect(leakyReluDerivative(-3.0), closeTo(0.01, tol));
      expect(leakyReluDerivative(0.0), closeTo(0.01, tol));
    });
  });

  group('tanh', () {
    test('matches reference values', () {
      expect(tanh(0.0), closeTo(0.0, tol));
      expect(tanh(1.0), closeTo(0.7615941559557649, tol));
      expect(tanh(-1.0), closeTo(-0.7615941559557649, tol));
    });
    test('saturates at extremes', () {
      expect(tanh(50.0), closeTo(1.0, tol));
      expect(tanh(-50.0), closeTo(-1.0, tol));
    });
    test('derivative matches reference values', () {
      expect(tanhDerivative(0.0), closeTo(1.0, tol));
      expect(tanhDerivative(1.0), closeTo(0.41997434161402614, tol));
    });
    test('agrees with the exponential definition on a sweep', () {
      for (var x = -6.0; x <= 6.0; x += 0.5) {
        final ex = math.exp(x), enx = math.exp(-x);
        final ref = (ex - enx) / (ex + enx);
        expect(tanh(x), closeTo(ref, 1e-12), reason: 'tanh($x)');
      }
    });
  });

  group('softplus', () {
    test('matches reference values', () {
      expect(softplus(0.0), closeTo(0.6931471805599453, tol));
      expect(softplus(1.0), closeTo(1.3132616875182228, tol));
      expect(softplus(-1.0), closeTo(0.31326168751822286, tol));
    });
    test('does not overflow for large input', () {
      expect(softplus(1000.0), greaterThan(999.0));
      expect(softplus(1000.0).isFinite, isTrue);
    });
    test('derivative equals sigmoid', () {
      expect(softplusDerivative(0.0), closeTo(0.5, tol));
      expect(softplusDerivative(1.0), closeTo(sigmoid(1.0), tol));
      expect(softplusDerivative(-1.0), closeTo(sigmoid(-1.0), tol));
    });
  });

  group('leakyReluSlope', () {
    test('is 0.01', () => expect(leakyReluSlope, equals(0.01)));
  });
}
