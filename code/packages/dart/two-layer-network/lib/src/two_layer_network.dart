import 'dart:math' as math;

const twoLayerNetworkVersion = '0.1.0';

enum ActivationName { linear, sigmoid }

typedef Matrix = List<List<double>>;

class Parameters {
  Parameters({
    required this.inputToHiddenWeights,
    required this.hiddenBiases,
    required this.hiddenToOutputWeights,
    required this.outputBiases,
  });

  final Matrix inputToHiddenWeights;
  final List<double> hiddenBiases;
  final Matrix hiddenToOutputWeights;
  final List<double> outputBiases;
}

class ForwardPass {
  ForwardPass({
    required this.hiddenRaw,
    required this.hiddenActivations,
    required this.outputRaw,
    required this.predictions,
  });

  final Matrix hiddenRaw;
  final Matrix hiddenActivations;
  final Matrix outputRaw;
  final Matrix predictions;
}

class TrainingStep {
  TrainingStep({
    required this.predictions,
    required this.errors,
    required this.outputDeltas,
    required this.hiddenDeltas,
    required this.hiddenToOutputWeightGradients,
    required this.outputBiasGradients,
    required this.inputToHiddenWeightGradients,
    required this.hiddenBiasGradients,
    required this.nextParameters,
    required this.loss,
  });

  final Matrix predictions;
  final Matrix errors;
  final Matrix outputDeltas;
  final Matrix hiddenDeltas;
  final Matrix hiddenToOutputWeightGradients;
  final List<double> outputBiasGradients;
  final Matrix inputToHiddenWeightGradients;
  final List<double> hiddenBiasGradients;
  final Parameters nextParameters;
  final double loss;
}

Parameters xorWarmStartParameters() => Parameters(
      inputToHiddenWeights: [
        [4.0, -4.0],
        [4.0, -4.0],
      ],
      hiddenBiases: [-2.0, 6.0],
      hiddenToOutputWeights: [
        [4.0],
        [4.0],
      ],
      outputBiases: [-6.0],
    );

ForwardPass forward(
  Matrix inputs,
  Parameters parameters, {
  ActivationName hiddenActivation = ActivationName.sigmoid,
  ActivationName outputActivation = ActivationName.sigmoid,
}) {
  final hiddenRaw = _addBiases(_dot(inputs, parameters.inputToHiddenWeights), parameters.hiddenBiases);
  final hiddenActivations = _applyActivation(hiddenRaw, hiddenActivation);
  final outputRaw = _addBiases(_dot(hiddenActivations, parameters.hiddenToOutputWeights), parameters.outputBiases);
  final predictions = _applyActivation(outputRaw, outputActivation);
  return ForwardPass(
    hiddenRaw: hiddenRaw,
    hiddenActivations: hiddenActivations,
    outputRaw: outputRaw,
    predictions: predictions,
  );
}

TrainingStep trainOneEpoch(
  Matrix inputs,
  Matrix targets,
  Parameters parameters,
  double learningRate, {
  ActivationName hiddenActivation = ActivationName.sigmoid,
  ActivationName outputActivation = ActivationName.sigmoid,
}) {
  final sampleCount = _validateMatrix('inputs', inputs).$1;
  final outputCount = _validateMatrix('targets', targets).$2;
  final pass = forward(inputs, parameters, hiddenActivation: hiddenActivation, outputActivation: outputActivation);
  final scale = 2.0 / (sampleCount * outputCount);
  final errors = List.generate(sampleCount, (_) => List.filled(outputCount, 0.0));
  final outputDeltas = List.generate(sampleCount, (_) => List.filled(outputCount, 0.0));
  for (var row = 0; row < sampleCount; row++) {
    for (var output = 0; output < outputCount; output++) {
      final error = pass.predictions[row][output] - targets[row][output];
      errors[row][output] = error;
      outputDeltas[row][output] = scale * error * _derivative(pass.outputRaw[row][output], pass.predictions[row][output], outputActivation);
    }
  }
  final h2oGradients = _dot(_transpose(pass.hiddenActivations), outputDeltas);
  final outputBiasGradients = _columnSums(outputDeltas);
  final hiddenErrors = _dot(outputDeltas, _transpose(parameters.hiddenToOutputWeights));
  final hiddenWidth = parameters.hiddenBiases.length;
  final hiddenDeltas = List.generate(sampleCount, (_) => List.filled(hiddenWidth, 0.0));
  for (var row = 0; row < sampleCount; row++) {
    for (var hidden = 0; hidden < hiddenWidth; hidden++) {
      hiddenDeltas[row][hidden] = hiddenErrors[row][hidden] *
          _derivative(pass.hiddenRaw[row][hidden], pass.hiddenActivations[row][hidden], hiddenActivation);
    }
  }
  final i2hGradients = _dot(_transpose(inputs), hiddenDeltas);
  final hiddenBiasGradients = _columnSums(hiddenDeltas);
  return TrainingStep(
    predictions: pass.predictions,
    errors: errors,
    outputDeltas: outputDeltas,
    hiddenDeltas: hiddenDeltas,
    hiddenToOutputWeightGradients: h2oGradients,
    outputBiasGradients: outputBiasGradients,
    inputToHiddenWeightGradients: i2hGradients,
    hiddenBiasGradients: hiddenBiasGradients,
    nextParameters: Parameters(
      inputToHiddenWeights: _subtractScaled(parameters.inputToHiddenWeights, i2hGradients, learningRate),
      hiddenBiases: _subtractScaledVector(parameters.hiddenBiases, hiddenBiasGradients, learningRate),
      hiddenToOutputWeights: _subtractScaled(parameters.hiddenToOutputWeights, h2oGradients, learningRate),
      outputBiases: _subtractScaledVector(parameters.outputBiases, outputBiasGradients, learningRate),
    ),
    loss: _mse(errors),
  );
}

(int, int) _validateMatrix(String name, Matrix matrix) {
  if (matrix.isEmpty) throw ArgumentError('$name must contain at least one row');
  final width = matrix.first.length;
  if (width == 0) throw ArgumentError('$name must contain at least one column');
  if (matrix.any((row) => row.length != width)) throw ArgumentError('$name must be rectangular');
  return (matrix.length, width);
}

Matrix _dot(Matrix left, Matrix right) {
  final leftShape = _validateMatrix('left', left);
  final rightShape = _validateMatrix('right', right);
  if (leftShape.$2 != rightShape.$1) throw ArgumentError('matrix shapes do not align');
  return List.generate(leftShape.$1, (row) {
    return List.generate(rightShape.$2, (col) {
      var total = 0.0;
      for (var k = 0; k < leftShape.$2; k++) {
        total += left[row][k] * right[k][col];
      }
      return total;
    });
  });
}

Matrix _transpose(Matrix matrix) {
  final shape = _validateMatrix('matrix', matrix);
  return List.generate(shape.$2, (col) => List.generate(shape.$1, (row) => matrix[row][col]));
}

Matrix _addBiases(Matrix matrix, List<double> biases) =>
    List.generate(matrix.length, (row) => List.generate(matrix[row].length, (col) => matrix[row][col] + biases[col]));

Matrix _applyActivation(Matrix matrix, ActivationName activation) =>
    List.generate(matrix.length, (row) => List.generate(matrix[row].length, (col) => _activate(matrix[row][col], activation)));

List<double> _columnSums(Matrix matrix) {
  final cols = _validateMatrix('matrix', matrix).$2;
  return List.generate(cols, (col) => matrix.fold(0.0, (sum, row) => sum + row[col]));
}

Matrix _subtractScaled(Matrix matrix, Matrix gradients, double learningRate) =>
    List.generate(matrix.length, (row) => List.generate(matrix[row].length, (col) => matrix[row][col] - learningRate * gradients[row][col]));

List<double> _subtractScaledVector(List<double> values, List<double> gradients, double learningRate) =>
    List.generate(values.length, (index) => values[index] - learningRate * gradients[index]);

double _mse(Matrix errors) {
  final values = errors.expand((row) => row).toList();
  return values.fold(0.0, (sum, value) => sum + value * value) / values.length;
}

double _activate(double value, ActivationName activation) {
  switch (activation) {
    case ActivationName.linear:
      return value;
    case ActivationName.sigmoid:
      if (value >= 0.0) return 1.0 / (1.0 + math.exp(-value));
      final z = math.exp(value);
      return z / (1.0 + z);
  }
}

double _derivative(double raw, double activated, ActivationName activation) {
  switch (activation) {
    case ActivationName.linear:
      return 1.0;
    case ActivationName.sigmoid:
      return activated * (1.0 - activated);
  }
}
