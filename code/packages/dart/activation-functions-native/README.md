# coding_adventures_activation_functions_native

**Native-through-Rust** Dart bindings for neural-network activation functions.
Same API as the pure-Dart
[`coding_adventures_activation_functions`](../activation-functions) package, but
every value is computed by the Rust `activation-functions` crate through a C
ABI, loaded via `dart:ffi`.

## The simplest native shape

Every function is a pure `double -> double`. There are **no strings, no byte
buffers, no opaque handles** — nothing to allocate or free. The C ABI passes and
returns `f64` by value:

```c
double af_sigmoid(double x);
double af_relu(double x);
double af_tanh(double x);
/* … 12 functions total, one per activation + derivative … */
```

This complements the other native shapes in the repo: `caesar-cipher-native`
(C strings) and `sha256-native`/`md5-native`/`sha1-native` (byte buffers +
opaque handles).

## Usage

```dart
import 'package:coding_adventures_activation_functions_native/coding_adventures_activation_functions_native.dart';

void main() {
  print(sigmoid(0.0)); // 0.5, computed in Rust
  print(tanh(1.0));    // 0.7615941559557649
}
```

## Building and testing

```
sh tools/run-tests.sh
```

builds the cdylib, sets `ACTIVATION_FUNCTIONS_NATIVE_PATH`, and runs `dart
test`. Windows CI is skipped (cdylib cross-compile out of scope); Linux and
macOS build and test.
