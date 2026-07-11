# coding_adventures_activation_functions

Neural-network **activation functions** and their derivatives, in pure Dart.

Dart port of the `activation-functions` package that already exists in Rust,
Java, Kotlin, and other languages in the monorepo; produces the same `f64`
values.

## What it provides

| Activation | Function | Derivative |
|---|---|---|
| Linear (identity) | `linear` | `linearDerivative` (≡ 1) |
| Sigmoid (logistic) | `sigmoid` | `sigmoidDerivative` |
| ReLU | `relu` | `reluDerivative` |
| Leaky ReLU | `leakyRelu` | `leakyReluDerivative` |
| Tanh | `tanh` | `tanhDerivative` |
| Softplus | `softplus` | `softplusDerivative` (≡ `sigmoid`) |

Plus the constant `leakyReluSlope` (`0.01`).

## Usage

```dart
import 'package:coding_adventures_activation_functions/coding_adventures_activation_functions.dart';

void main() {
  print(sigmoid(0.0));           // 0.5
  print(relu(-3.0));             // 0.0
  print(leakyRelu(-3.0));        // -0.03
  print(tanh(1.0));              // 0.7615941559557649
  print(sigmoidDerivative(0.0)); // 0.25
}
```

## Numerical notes

- `sigmoid` guards against `exp` overflow: it underflows to 0 below `x = -709`
  and saturates to 1 above `x = 709`.
- `dart:math` has no `tanh`; this uses the stable identity
  `tanh(x) = (e^{2x} − 1)/(e^{2x} + 1)` and saturates beyond `|x| = 20`.
- `softplus` uses the stable form `ln(1 + e^{−|x|}) + max(x, 0)` with a local
  `log1p` so it never overflows for large positive `x`.

## Running the tests

```
dart pub get
dart test
```
