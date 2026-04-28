import 'dart:math' as math;

const singleLayerNetworkVersion = '0.1.0';

enum ActivationName { linear, sigmoid }

typedef Matrix = List<List<double>>;

class TrainingStep {
  TrainingStep({
    required this.predictions,
    required this.errors,
    required this.weightGradients,
    required this.biasGradients,
    required this.nextWeights,
    required this.nextBiases,
    required this.loss,
  });

  final Matrix predictions;
  final Matrix errors;
  final Matrix weightGradients;
  final List<double> biasGradients;
  final Matrix nextWeights;
  final List<double> nextBiases;
  final double loss;
}

class SingleLayerNetwork {
  SingleLayerNetwork(int inputCount, int outputCount, {this.activation = ActivationName.linear})
      : weights = List.generate(inputCount, (_) => List.filled(outputCount, 0.0)),
        biases = List.filled(outputCount, 0.0);

  Matrix weights;
  List<double> biases;
  final ActivationName activation;

  Matrix predict(Matrix inputs) => predictWithParameters(inputs, weights, biases, activation: activation);

  List<TrainingStep> fit(Matrix inputs, Matrix targets, {double learningRate = 0.05, int epochs = 100}) {
    final history = <TrainingStep>[];
    for (var epoch = 0; epoch < epochs; epoch++) {
      final step = trainOneEpochWithMatrices(inputs, targets, weights, biases, learningRate, activation: activation);
      weights = step.nextWeights;
      biases = step.nextBiases;
      history.add(step);
    }
    return history;
  }
}

Matrix predictWithParameters(Matrix inputs, Matrix weights, List<double> biases, {ActivationName activation = ActivationName.linear}) {
  final inputShape = _validateMatrix('inputs', inputs);
  final weightShape = _validateMatrix('weights', weights);
  final sampleCount = inputShape.$1;
  final inputCount = inputShape.$2;
  final outputCount = weightShape.$2;
  if (inputCount != weightShape.$1) {
    throw ArgumentError('input column count must match weight row count');
  }
  if (biases.length != outputCount) {
    throw ArgumentError('bias count must match output count');
  }

  return List.generate(sampleCount, (row) {
    return List.generate(outputCount, (output) {
      var total = biases[output];
      for (var input = 0; input < inputCount; input++) {
        total += inputs[row][input] * weights[input][output];
      }
      return _activate(total, activation);
    });
  });
}

TrainingStep trainOneEpochWithMatrices(
  Matrix inputs,
  Matrix targets,
  Matrix weights,
  List<double> biases,
  double learningRate, {
  ActivationName activation = ActivationName.linear,
}) {
  final inputShape = _validateMatrix('inputs', inputs);
  final targetShape = _validateMatrix('targets', targets);
  final weightShape = _validateMatrix('weights', weights);
  final sampleCount = inputShape.$1;
  final inputCount = inputShape.$2;
  final outputCount = targetShape.$2;
  if (targetShape.$1 != sampleCount) {
    throw ArgumentError('inputs and targets must have the same row count');
  }
  if (weightShape.$1 != inputCount || weightShape.$2 != outputCount) {
    throw ArgumentError('weights must be shaped input_count x output_count');
  }
  if (biases.length != outputCount) {
    throw ArgumentError('bias count must match output count');
  }

  final predictions = predictWithParameters(inputs, weights, biases, activation: activation);
  final scale = 2.0 / (sampleCount * outputCount);
  final errors = List.generate(sampleCount, (_) => List.filled(outputCount, 0.0));
  final deltas = List.generate(sampleCount, (_) => List.filled(outputCount, 0.0));
  var lossTotal = 0.0;
  for (var row = 0; row < sampleCount; row++) {
    for (var output = 0; output < outputCount; output++) {
      final error = predictions[row][output] - targets[row][output];
      errors[row][output] = error;
      deltas[row][output] = scale * error * _derivativeFromOutput(predictions[row][output], activation);
      lossTotal += error * error;
    }
  }

  final weightGradients = List.generate(inputCount, (_) => List.filled(outputCount, 0.0));
  final nextWeights = List.generate(inputCount, (_) => List.filled(outputCount, 0.0));
  for (var input = 0; input < inputCount; input++) {
    for (var output = 0; output < outputCount; output++) {
      for (var row = 0; row < sampleCount; row++) {
        weightGradients[input][output] += inputs[row][input] * deltas[row][output];
      }
      nextWeights[input][output] = weights[input][output] - learningRate * weightGradients[input][output];
    }
  }

  final biasGradients = List.filled(outputCount, 0.0);
  final nextBiases = List.filled(outputCount, 0.0);
  for (var output = 0; output < outputCount; output++) {
    for (var row = 0; row < sampleCount; row++) {
      biasGradients[output] += deltas[row][output];
    }
    nextBiases[output] = biases[output] - learningRate * biasGradients[output];
  }

  return TrainingStep(
    predictions: predictions,
    errors: errors,
    weightGradients: weightGradients,
    biasGradients: biasGradients,
    nextWeights: nextWeights,
    nextBiases: nextBiases,
    loss: lossTotal / (sampleCount * outputCount),
  );
}

(int, int) _validateMatrix(String name, Matrix matrix) {
  if (matrix.isEmpty) throw ArgumentError('$name must contain at least one row');
  final width = matrix.first.length;
  if (width == 0) throw ArgumentError('$name must contain at least one column');
  if (matrix.any((row) => row.length != width)) throw ArgumentError('$name must be rectangular');
  return (matrix.length, width);
}

double _activate(double value, ActivationName activation) {
  switch (activation) {
    case ActivationName.linear:
      return value;
    case ActivationName.sigmoid:
      if (value >= 0.0) {
        return 1.0 / (1.0 + math.exp(-value));
      }
      final z = math.exp(value);
      return z / (1.0 + z);
  }
}

double _derivativeFromOutput(double output, ActivationName activation) {
  switch (activation) {
    case ActivationName.linear:
      return 1.0;
    case ActivationName.sigmoid:
      return output * (1.0 - output);
  }
}
