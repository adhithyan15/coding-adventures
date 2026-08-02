import 'dart:math' as math;

const double epsilon = 1e-7;

double mse(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  var total = 0.0;
  for (var index = 0; index < yTrue.length; index++) {
    final difference = yTrue[index] - yPred[index];
    total += difference * difference;
  }
  return total / yTrue.length;
}

double mae(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  var total = 0.0;
  for (var index = 0; index < yTrue.length; index++) {
    total += (yTrue[index] - yPred[index]).abs();
  }
  return total / yTrue.length;
}

double bce(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  var total = 0.0;
  for (var index = 0; index < yTrue.length; index++) {
    final prediction = _clamp(yPred[index]);
    total += yTrue[index] * math.log(prediction) +
        (1.0 - yTrue[index]) * math.log(1.0 - prediction);
  }
  return -total / yTrue.length;
}

double cce(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  var total = 0.0;
  for (var index = 0; index < yTrue.length; index++) {
    total += yTrue[index] * math.log(_clamp(yPred[index]));
  }
  return -total / yTrue.length;
}

double binaryCrossEntropy(List<double> yTrue, List<double> yPred) =>
    bce(yTrue, yPred);

double categoricalCrossEntropy(List<double> yTrue, List<double> yPred) =>
    cce(yTrue, yPred);

List<double> mseDerivative(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  final scale = 2.0 / yTrue.length;
  return List.generate(
    yTrue.length,
    (index) => scale * (yPred[index] - yTrue[index]),
  );
}

List<double> maeDerivative(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  final scale = 1.0 / yTrue.length;
  return List.generate(yTrue.length, (index) {
    final difference = yPred[index] - yTrue[index];
    if (difference > 0) return scale;
    if (difference < 0) return -scale;
    return 0.0;
  });
}

List<double> bceDerivative(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  final scale = 1.0 / yTrue.length;
  return List.generate(yTrue.length, (index) {
    final prediction = _clamp(yPred[index]);
    return scale *
        ((prediction - yTrue[index]) / (prediction * (1.0 - prediction)));
  });
}

List<double> cceDerivative(List<double> yTrue, List<double> yPred) {
  _validate(yTrue, yPred);
  final scale = -1.0 / yTrue.length;
  return List.generate(
    yTrue.length,
    (index) => scale * (yTrue[index] / _clamp(yPred[index])),
  );
}

double _clamp(double value) => value.clamp(epsilon, 1.0 - epsilon).toDouble();

void _validate(List<double> yTrue, List<double> yPred) {
  if (yTrue.isEmpty || yTrue.length != yPred.length) {
    throw ArgumentError('inputs must have the same non-zero length');
  }
}
