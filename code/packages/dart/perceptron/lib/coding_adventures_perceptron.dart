/// Logistic perceptron trained by gradient descent.
///
/// The Dart lane of `perceptron`: [Perceptron.fit] runs full-batch
/// gradient descent on binary cross-entropy, and [Perceptron.predict]
/// scores rows through the fitted weights.
library coding_adventures_perceptron;

export 'src/perceptron.dart';
