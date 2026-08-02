# Loss Functions (Dart)

A dependency-free Dart implementation of the losses and analytical derivatives
specified by [ML01](../../../specs/ML01-loss-functions.md).

The canonical API provides `mse`, `mae`, `bce`, `cce`, and corresponding
`*Derivative` functions. Descriptive binary and categorical cross-entropy
aliases are also available. Every function requires equal, non-empty vectors;
cross-entropy inputs are clamped to `1e-7` through `1 - 1e-7` for numerical
stability, and inputs are never mutated.

```dart
import 'package:coding_adventures_loss_functions/loss_functions.dart';

final loss = mse([1.0, 0.0], [0.9, 0.1]);
final gradient = mseDerivative([1.0, 0.0], [0.9, 0.1]);
```

Run the package checks with:

```sh
dart pub get
dart analyze
dart test
```
