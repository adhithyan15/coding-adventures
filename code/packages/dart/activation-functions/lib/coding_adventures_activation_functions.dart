/// Neural-network activation functions and their derivatives, in pure Dart.
///
/// Provides the identity, sigmoid, ReLU, leaky ReLU, tanh, and softplus
/// activations — each paired with its derivative for backpropagation.
///
/// ```dart
/// import 'package:coding_adventures_activation_functions/coding_adventures_activation_functions.dart';
///
/// void main() {
///   print(sigmoid(0.0));            // 0.5
///   print(relu(-3.0));              // 0.0
///   print(tanh(1.0));               // 0.7615941559557649
///   print(sigmoidDerivative(0.0));  // 0.25
/// }
/// ```
library coding_adventures_activation_functions;

export 'src/activation_functions.dart';
