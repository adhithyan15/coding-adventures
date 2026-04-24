/// Coefficient-array polynomial arithmetic.
///
/// Polynomials are represented as `List<num>` where index `i` holds the
/// coefficient of `x^i` (little-endian: constant term first).
///
/// The zero polynomial is the empty list `[]`.
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_polynomial/coding_adventures_polynomial.dart';
///
/// void main() {
///   // 1 + 2x + 3x²  plus  4 + 5x  =  5 + 7x + 3x²
///   final sum = polynomialAdd([1, 2, 3], [4, 5]);
///   print(sum);  // → [5, 7, 3]
///
///   // (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
///   final product = polynomialMultiply([1, 2], [3, 4]);
///   print(product);  // → [3, 10, 8]
///
///   // Evaluate 3 + x + 2x² at x = 4  →  39
///   print(polynomialEvaluate([3, 1, 2], 4));  // → 39
/// }
/// ```
library coding_adventures_polynomial;

export 'src/polynomial.dart';
