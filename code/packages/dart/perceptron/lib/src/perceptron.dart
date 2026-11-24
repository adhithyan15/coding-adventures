/// Logistic perceptron trained by gradient descent.
///
/// The Dart lane of the Rust `perceptron` crate: [Perceptron.fit] runs
/// full-batch gradient descent on binary cross-entropy through a sigmoid
/// unit ([epochs] inclusive, weights from zero), and [Perceptron.predict]
/// scores rows through the fitted weights.
///
/// The training loop calls only the in-lane matrix, activation, and loss
/// packages, exactly as the reference calls its three crates. Progress
/// logging goes to the optional [Perceptron.fit] `log` callback (silent
/// by default, keeping the package pure); pass `log: print` for the
/// reference's exact line format. Predicting before fitting throws
/// [StateError] (the reference panics on the same bug).
library;

import 'package:coding_adventures_activation_functions/coding_adventures_activation_functions.dart'
    show sigmoid, sigmoidDerivative;
import 'package:coding_adventures_loss_functions/loss_functions.dart'
    show bce, bceDerivative;
import 'package:coding_adventures_matrix/matrix.dart' show Matrix;

/// A single logistic unit: `sigmoid(X·w + b)` fitted by gradient descent.
class Perceptron {
  /// Step size applied to every gradient update.
  double learningRate;

  /// Training passes over the data (inclusive: `epochs + 1` updates).
  int epochs;

  /// Fitted weight column (features × 1), or null before [fit].
  Matrix? weights;

  /// Fitted bias term.
  double bias;

  /// Creates an unfitted perceptron.
  Perceptron(this.learningRate, this.epochs) : bias = 0.0;

  /// Fits weights and bias to [xData] (rows × features) and [yData]
  /// (rows × 1 labels) by full-batch gradient descent.
  ///
  /// Weights restart from zero and the bias from 0.0 on every call, so
  /// refitting is deterministic. Every [logSteps]-th epoch (counting the
  /// zeroth) reports `Epoch … | BCE Loss: … | Bias: …` through [log]
  /// (reference format; pass `print` to reproduce it on stdout).
  ///
  /// Throws [ArgumentError] (via the matrix/loss lanes) on empty,
  /// ragged, or dimension-mismatched inputs; a zero [logSteps] throws
  /// [UnsupportedError], as the reference panics dividing by it.
  void fit(
    List<List<double>> xData,
    List<List<double>> yData,
    int logSteps, {
    void Function(String line)? log,
  }) {
    final features = Matrix(xData);
    final trueLabels = Matrix(yData);
    var w = Matrix.zeros(features.cols, 1);
    weights = w;
    bias = 0.0;

    for (var epoch = 0; epoch <= epochs; epoch++) {
      final raw = features.dot(w).map((value) => value + bias);

      final linearProbs = List<double>.filled(features.rows, 0.0);
      final linearTruth = List<double>.filled(features.rows, 0.0);
      for (var i = 0; i < features.rows; i++) {
        linearProbs[i] = sigmoid(raw.data[i][0]);
        linearTruth[i] = trueLabels.data[i][0];
      }

      final logLoss = bce(linearTruth, linearProbs);
      final lossGrad = bceDerivative(linearTruth, linearProbs);

      final gradData = <List<double>>[];
      var biasGrad = 0.0;
      for (var i = 0; i < features.rows; i++) {
        final actGrad = sigmoidDerivative(raw.data[i][0]);
        final combined = lossGrad[i] * actGrad;
        gradData.add([combined]);
        biasGrad += combined;
      }

      final gradMatrix = Matrix(gradData);
      final weightGrads = features.transpose().dot(gradMatrix);
      final scaledWeights = weightGrads.scale(learningRate);
      w = w.subtract(scaledWeights);
      weights = w;
      bias -= biasGrad * learningRate;

      if (epoch % logSteps == 0) {
        log?.call('Epoch ${epoch.toString().padLeft(4)} | '
            'BCE Loss: ${logLoss.toStringAsFixed(4)} | '
            'Bias: ${bias.toStringAsFixed(2)}');
      }
    }
  }

  /// Scores each row of [xData], returning one probability per row.
  ///
  /// Throws [StateError] before the first [fit].
  List<double> predict(List<List<double>> xData) {
    final w = weights;
    if (w == null) {
      throw StateError('predict() requires fit() to run first');
    }
    final features = Matrix(xData);
    final raw = features.dot(w).map((value) => value + bias);
    return [for (var i = 0; i < features.rows; i++) sigmoid(raw.data[i][0])];
  }
}
