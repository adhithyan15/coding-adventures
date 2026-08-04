import 'dart:math' as math;

import 'package:coding_adventures_activation_functions_native/coding_adventures_activation_functions_native.dart';
import 'package:test/test.dart';

const tol = 1e-12;

/// Exercises the Rust activation functions through dart:ffi, asserting the same
/// reference values as the pure-Dart port.
void main() {
  test('linear / derivative', () {
    expect(linear(-3.0), closeTo(-3.0, tol));
    expect(linear(5.0), closeTo(5.0, tol));
    expect(linearDerivative(5.0), closeTo(1.0, tol));
  });

  test('sigmoid / derivative + overflow guards', () {
    expect(sigmoid(0.0), closeTo(0.5, tol));
    expect(sigmoid(1.0), closeTo(0.7310585786300049, tol));
    expect(sigmoid(-1.0), closeTo(0.2689414213699951, tol));
    expect(sigmoid(-710.0), closeTo(0.0, tol));
    expect(sigmoid(710.0), closeTo(1.0, tol));
    expect(sigmoidDerivative(0.0), closeTo(0.25, tol));
    expect(sigmoidDerivative(1.0), closeTo(0.19661193324148185, tol));
  });

  test('relu / derivative', () {
    expect(relu(5.0), closeTo(5.0, tol));
    expect(relu(-3.0), closeTo(0.0, tol));
    expect(reluDerivative(5.0), closeTo(1.0, tol));
    expect(reluDerivative(-3.0), closeTo(0.0, tol));
    expect(reluDerivative(0.0), closeTo(0.0, tol));
  });

  test('leakyRelu / derivative', () {
    expect(leakyRelu(5.0), closeTo(5.0, tol));
    expect(leakyRelu(-3.0), closeTo(-0.03, tol));
    expect(leakyReluDerivative(-3.0), closeTo(0.01, tol));
    expect(leakyReluDerivative(0.0), closeTo(0.01, tol));
  });

  test('tanh / derivative', () {
    expect(tanh(0.0), closeTo(0.0, tol));
    expect(tanh(1.0), closeTo(0.7615941559557649, tol));
    expect(tanh(-1.0), closeTo(-0.7615941559557649, tol));
    expect(tanh(50.0), closeTo(1.0, tol));
    expect(tanhDerivative(0.0), closeTo(1.0, tol));
    expect(tanhDerivative(1.0), closeTo(0.41997434161402614, tol));
  });

  test('softplus / derivative', () {
    expect(softplus(0.0), closeTo(0.6931471805599453, tol));
    expect(softplus(1.0), closeTo(1.3132616875182228, tol));
    expect(softplus(-1.0), closeTo(0.31326168751822286, tol));
    expect(softplus(1000.0), greaterThan(999.0));
    expect(softplus(1000.0).isFinite, isTrue);
    expect(softplusDerivative(1.0), closeTo(sigmoid(1.0), tol));
  });

  test('parity: native tanh agrees with the exponential definition', () {
    for (var x = -6.0; x <= 6.0; x += 0.5) {
      final ex = math.exp(x), enx = math.exp(-x);
      expect(tanh(x), closeTo((ex - enx) / (ex + enx), 1e-12), reason: 'tanh($x)');
    }
  });
}
