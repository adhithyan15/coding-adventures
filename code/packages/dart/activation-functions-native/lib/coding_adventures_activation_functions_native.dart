/// Neural-network activation functions — **native-through-Rust** Dart bindings.
///
/// Same API as the pure-Dart `coding_adventures_activation_functions` package,
/// but every value is computed by the Rust `activation-functions` crate through
/// a C ABI (`dart:ffi`). Each function is a pure `double -> double`, so this is
/// the simplest native binding in the repo — no buffers, handles, or allocation.
///
/// ```dart
/// import 'package:coding_adventures_activation_functions_native/coding_adventures_activation_functions_native.dart';
///
/// void main() {
///   print(sigmoid(0.0)); // 0.5, computed in Rust
///   print(tanh(1.0));    // 0.7615941559557649
/// }
/// ```
///
/// The shared library is located via `ACTIVATION_FUNCTIONS_NATIVE_PATH` (an
/// absolute path) or the platform default name; `tools/run-tests.sh` sets it.
library coding_adventures_activation_functions_native;

import 'src/ffi.dart' as ffi;

/// Identity activation (executed in Rust).
double linear(double x) => ffi.linear(x);

/// Derivative of [linear] (≡ 1).
double linearDerivative(double x) => ffi.linearDerivative(x);

/// Logistic sigmoid `1 / (1 + e^-x)` (executed in Rust).
double sigmoid(double x) => ffi.sigmoid(x);

/// Derivative of [sigmoid].
double sigmoidDerivative(double x) => ffi.sigmoidDerivative(x);

/// ReLU `max(0, x)` (executed in Rust).
double relu(double x) => ffi.relu(x);

/// Derivative of [relu].
double reluDerivative(double x) => ffi.reluDerivative(x);

/// Leaky ReLU (executed in Rust).
double leakyRelu(double x) => ffi.leakyRelu(x);

/// Derivative of [leakyRelu].
double leakyReluDerivative(double x) => ffi.leakyReluDerivative(x);

/// Hyperbolic tangent (executed in Rust).
double tanh(double x) => ffi.tanh(x);

/// Derivative of [tanh].
double tanhDerivative(double x) => ffi.tanhDerivative(x);

/// Softplus `ln(1 + e^x)` (executed in Rust).
double softplus(double x) => ffi.softplus(x);

/// Derivative of [softplus] (≡ [sigmoid]).
double softplusDerivative(double x) => ffi.softplusDerivative(x);
