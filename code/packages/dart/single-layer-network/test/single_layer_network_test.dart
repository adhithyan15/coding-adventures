import 'package:test/test.dart';
import 'package:coding_adventures_single_layer_network/coding_adventures_single_layer_network.dart';

void main() {
  test('one epoch exposes matrix gradients', () {
    final step = trainOneEpochWithMatrices(
      [
        [1.0, 2.0],
      ],
      [
        [3.0, 5.0],
      ],
      [
        [0.0, 0.0],
        [0.0, 0.0],
      ],
      [0.0, 0.0],
      0.1,
    );

    expect(step.weightGradients[0][0], closeTo(-3.0, 1e-6));
    expect(step.weightGradients[1][1], closeTo(-10.0, 1e-6));
    expect(step.nextWeights[0][0], closeTo(0.3, 1e-6));
    expect(step.nextWeights[1][1], closeTo(1.0, 1e-6));
  });

  test('fit learns m inputs to n outputs', () {
    final model = SingleLayerNetwork(3, 2);
    final history = model.fit(
      [
        [0.0, 0.0, 1.0],
        [1.0, 2.0, 1.0],
        [2.0, 1.0, 1.0],
      ],
      [
        [1.0, -1.0],
        [3.0, 2.0],
        [4.0, 1.0],
      ],
      learningRate: 0.05,
      epochs: 500,
    );
    expect(history.last.loss, lessThan(history.first.loss));
    expect(model.predict([
      [1.0, 1.0, 1.0],
    ]).first.length, equals(2));
  });
}
