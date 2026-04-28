import 'package:coding_adventures_two_layer_network/coding_adventures_two_layer_network.dart';
import 'package:test/test.dart';

void main() {
  final inputs = [
    [0.0, 0.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [1.0, 1.0],
  ];
  final targets = [
    [0.0],
    [1.0],
    [1.0],
    [0.0],
  ];

  test('forward pass exposes hidden activations', () {
    final pass = forward(inputs, xorWarmStartParameters());

    expect(pass.hiddenActivations.length, 4);
    expect(pass.hiddenActivations.first.length, 2);
    expect(pass.predictions[1][0], greaterThan(0.7));
    expect(pass.predictions[0][0], lessThan(0.3));
  });

  test('training step exposes both layer gradients', () {
    final step = trainOneEpoch(inputs, targets, xorWarmStartParameters(), 0.5);

    expect(step.inputToHiddenWeightGradients.length, 2);
    expect(step.inputToHiddenWeightGradients.first.length, 2);
    expect(step.hiddenToOutputWeightGradients.length, 2);
    expect(step.hiddenToOutputWeightGradients.first.length, 1);
  });
}
