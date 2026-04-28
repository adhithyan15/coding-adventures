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

  test('hidden layer teaching examples run one training step', () {
    final cases = [
      _ExampleCase('XNOR', inputs, [
        [1.0],
        [0.0],
        [0.0],
        [1.0],
      ], 3),
      _ExampleCase(
        'absolute value',
        [
          [-1.0],
          [-0.5],
          [0.0],
          [0.5],
          [1.0],
        ],
        [
          [1.0],
          [0.5],
          [0.0],
          [0.5],
          [1.0],
        ],
        4,
      ),
      _ExampleCase(
        'piecewise pricing',
        [
          [0.1],
          [0.3],
          [0.5],
          [0.7],
          [0.9],
        ],
        [
          [0.12],
          [0.25],
          [0.55],
          [0.88],
          [0.88],
        ],
        4,
      ),
      _ExampleCase(
        'circle classifier',
        [
          [0.0, 0.0],
          [0.5, 0.0],
          [1.0, 1.0],
          [-0.5, 0.5],
          [-1.0, 0.0],
        ],
        [
          [1.0],
          [1.0],
          [0.0],
          [1.0],
          [0.0],
        ],
        5,
      ),
      _ExampleCase(
        'two moons',
        [
          [1.0, 0.0],
          [0.0, 0.5],
          [0.5, 0.85],
          [0.5, -0.35],
          [-1.0, 0.0],
          [2.0, 0.5],
        ],
        [
          [0.0],
          [1.0],
          [0.0],
          [1.0],
          [0.0],
          [1.0],
        ],
        5,
      ),
      _ExampleCase(
        'interaction features',
        [
          [0.2, 0.25, 0.0],
          [0.6, 0.5, 1.0],
          [1.0, 0.75, 1.0],
          [1.0, 1.0, 0.0],
        ],
        [
          [0.08],
          [0.72],
          [0.96],
          [0.76],
        ],
        5,
      ),
    ];

    for (final example in cases) {
      final step = trainOneEpoch(
        example.inputs,
        example.targets,
        _sampleParameters(example.inputs.first.length, example.hiddenCount),
        0.4,
      );

      expect(step.loss, greaterThanOrEqualTo(0.0), reason: example.name);
      expect(
        step.inputToHiddenWeightGradients.length,
        example.inputs.first.length,
        reason: example.name,
      );
      expect(
        step.hiddenToOutputWeightGradients.length,
        example.hiddenCount,
        reason: example.name,
      );
    }
  });
}

Parameters _sampleParameters(int inputCount, int hiddenCount) => Parameters(
  inputToHiddenWeights: List.generate(
    inputCount,
    (feature) => List.generate(
      hiddenCount,
      (hidden) => 0.17 * (feature + 1) - 0.11 * (hidden + 1),
    ),
  ),
  hiddenBiases: List.generate(hiddenCount, (hidden) => 0.05 * (hidden - 1)),
  hiddenToOutputWeights: List.generate(
    hiddenCount,
    (hidden) => [0.13 * (hidden + 1) - 0.25],
  ),
  outputBiases: [0.02],
);

class _ExampleCase {
  _ExampleCase(this.name, this.inputs, this.targets, this.hiddenCount);

  final String name;
  final Matrix inputs;
  final Matrix targets;
  final int hiddenCount;
}
