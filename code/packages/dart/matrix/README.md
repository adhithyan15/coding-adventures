# Matrix (Dart)

An immutable Dart implementation of the matrix contracts in
[ML03](../../../specs/ML03-matrix.md) and
[ML03 extensions](../../../specs/ML03-matrix-extensions.md).

`Matrix` accepts a scalar, numeric row vector, or rectangular numeric grid and
deep-copies it into immutable `double` data. The public API includes factories,
matrix and scalar arithmetic, dot products, reductions, element-wise math,
reshape and slice operations, and exact or tolerance-based comparison. Invalid
shapes, dimensions, and indices throw `ArgumentError` or `RangeError`.

```dart
import 'package:coding_adventures_matrix/matrix.dart';

final left = Matrix([[1, 2], [3, 4]]);
final right = Matrix.identity(2);
final product = left.dot(right);
print(product.data); // [[1.0, 2.0], [3.0, 4.0]]
```

Run the package checks with:

```sh
dart pub get
dart analyze
dart test
```
