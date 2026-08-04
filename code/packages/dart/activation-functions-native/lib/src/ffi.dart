// ffi.dart — dart:ffi bindings for the activation-functions-native Rust cdylib.
//
// The simplest FFI shape in the repo: every function is `double f(double)`.
// No allocation, no buffers, no handles — nothing to free. The library is
// located via ACTIVATION_FUNCTIONS_NATIVE_PATH (absolute) or the platform
// default name.

import 'dart:ffi';
import 'dart:io';

typedef _DoubleToDoubleC = Double Function(Double);
typedef _DoubleToDoubleDart = double Function(double);

final DynamicLibrary _lib = _load();

DynamicLibrary _load() {
  final envPath = Platform.environment['ACTIVATION_FUNCTIONS_NATIVE_PATH'];
  if (envPath != null && envPath.isNotEmpty) {
    if (!_isAbsolute(envPath)) {
      throw ArgumentError(
          'ACTIVATION_FUNCTIONS_NATIVE_PATH must be an absolute path, got: $envPath');
    }
    return DynamicLibrary.open(envPath);
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open('libactivation_functions_native.dylib');
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('activation_functions_native.dll');
  }
  return DynamicLibrary.open('libactivation_functions_native.so');
}

bool _isAbsolute(String p) =>
    p.startsWith('/') || RegExp(r'^[A-Za-z]:[\\/]').hasMatch(p);

_DoubleToDoubleDart _fn(String symbol) =>
    _lib.lookupFunction<_DoubleToDoubleC, _DoubleToDoubleDart>(symbol);

final linear = _fn('af_linear');
final linearDerivative = _fn('af_linear_derivative');
final sigmoid = _fn('af_sigmoid');
final sigmoidDerivative = _fn('af_sigmoid_derivative');
final relu = _fn('af_relu');
final reluDerivative = _fn('af_relu_derivative');
final leakyRelu = _fn('af_leaky_relu');
final leakyReluDerivative = _fn('af_leaky_relu_derivative');
final tanh = _fn('af_tanh');
final tanhDerivative = _fn('af_tanh_derivative');
final softplus = _fn('af_softplus');
final softplusDerivative = _fn('af_softplus_derivative');
