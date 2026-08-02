import 'package:coding_adventures_loss_functions/loss_functions.dart';
import 'package:test/test.dart';

const tolerance = 1e-9;

void main() {
  final yTrue = [1.0, 0.0, 0.0];
  final yPred = [0.9, 0.1, 0.2];

  group('shared parity vectors', () {
    test('regression losses match ML01', () {
      expect(mse(yTrue, yPred), closeTo(0.02, tolerance));
      expect(mae(yTrue, yPred), closeTo(0.1333333333, tolerance));
    });

    test('cross-entropy losses match ML01', () {
      expect(
        bce([1.0, 0.0, 1.0], [0.9, 0.1, 0.8]),
        closeTo(0.1446215275, tolerance),
      );
      expect(
        cce([1.0, 0.0, 0.0], [0.8, 0.1, 0.1]),
        closeTo(0.07438118377140324, tolerance),
      );
      expect(
        binaryCrossEntropy([1.0, 0.0], [0.9, 0.1]),
        closeTo(bce([1.0, 0.0], [0.9, 0.1]), tolerance),
      );
      expect(
        categoricalCrossEntropy([1.0, 0.0], [0.9, 0.1]),
        closeTo(cce([1.0, 0.0], [0.9, 0.1]), tolerance),
      );
    });
  });

  group('derivatives', () {
    test('mse and mae derivatives match the analytical vectors', () {
      expect(mseDerivative(yTrue, yPred), everyElement(isA<double>()));
      expect(mseDerivative(yTrue, yPred)[0], closeTo(-0.0666666667, tolerance));
      expect(mseDerivative(yTrue, yPred)[1], closeTo(0.0666666667, tolerance));
      expect(mseDerivative(yTrue, yPred)[2], closeTo(0.1333333333, tolerance));
      expect(maeDerivative(yTrue, yPred), equals([-1 / 3, 1 / 3, 1 / 3]));
    });

    test('cross-entropy derivatives clamp consistently', () {
      final bceGradient = bceDerivative([1.0, 0.0, 1.0], [0.9, 0.1, 0.8]);
      expect(bceGradient[0], closeTo(-0.3703703704, tolerance));
      expect(bceGradient[1], closeTo(0.3703703704, tolerance));
      expect(bceGradient[2], closeTo(-0.4166666667, tolerance));
      final cceGradient = cceDerivative([1.0, 0.0, 0.0], [0.8, 0.1, 0.1]);
      expect(cceGradient[0], closeTo(-0.4166666667, tolerance));
      expect(cceGradient[1], equals(0.0));
      expect(cceGradient[2], equals(0.0));
      expect(bce([1.0, 0.0], [0.0, 1.0]).isFinite, isTrue);
      expect(cce([1.0, 0.0], [0.0, 1.0]).isFinite, isTrue);
    });
  });

  group('validation and purity', () {
    test('rejects empty and mismatched inputs in every family', () {
      for (final loss in [mse, mae, bce, cce]) {
        expect(() => loss([], []), throwsArgumentError);
        expect(() => loss([1.0], [1.0, 2.0]), throwsArgumentError);
      }
      for (final derivative in [
        mseDerivative,
        maeDerivative,
        bceDerivative,
        cceDerivative,
      ]) {
        expect(() => derivative([], []), throwsArgumentError);
        expect(() => derivative([1.0], [1.0, 2.0]), throwsArgumentError);
      }
    });

    test('does not mutate inputs and returns fresh gradients', () {
      final truth = [1.0, 0.0];
      final predictions = [0.9, 0.1];
      final first = mseDerivative(truth, predictions);
      final second = mseDerivative(truth, predictions);
      first[0] = 99.0;
      expect(second[0], closeTo(-0.1, tolerance));
      expect(truth, equals([1.0, 0.0]));
      expect(predictions, equals([0.9, 0.1]));
    });
  });
}
