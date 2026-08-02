# Feature Normalization (Dart)

A dependency-free Dart implementation of the standard and min-max feature
normalization contracts in
[ML05](../../../specs/ML05-feature-normalization-and-learning-rate-sweeps.md).

`fitStandardScaler` uses population variance and returns immutable means and
standard deviations. `fitMinMaxScaler` returns immutable minima and maxima.
Their transform functions accept any row count with a matching feature width,
return fresh rows, and map constant columns to zero. Empty or ragged matrices
and incompatible scaler widths throw `ArgumentError`.

```dart
import 'package:coding_adventures_feature_normalization/feature_normalization.dart';

final rows = [[1.0, 10.0], [3.0, 20.0]];
final scaler = fitStandardScaler(rows);
final normalized = transformStandard(rows, scaler);
```

Run the package checks with:

```sh
dart pub get
dart analyze
dart test
```
