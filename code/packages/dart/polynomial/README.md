# coding_adventures_polynomial

Coefficient-array polynomial arithmetic for Reed-Solomon error correction.

## Representation

A polynomial is a `List<num>` where **index `i` holds the coefficient of `x^i`**:

```
[3, 0, 2]  →  3 + 0·x + 2·x²  =  3 + 2x²
[]         →  the zero polynomial (degree = -1)
```

This **little-endian** representation (constant term first) makes term-by-term
addition and Horner evaluation natural and efficient.

## Usage

```dart
import 'package:coding_adventures_polynomial/coding_adventures_polynomial.dart';

// Add: [1+2x+3x²] + [4+5x] = [5+7x+3x²]
final sum = polynomialAdd([1, 2, 3], [4, 5]);
print(sum);  // → [5, 7, 3]

// Multiply: (1+2x)(3+4x) = 3 + 10x + 8x²
final product = polynomialMultiply([1, 2], [3, 4]);
print(product);  // → [3, 10, 8]

// Divide: (5+x+3x²+2x³) / (2+x)
final (q, r) = polynomialDivmod([5, 1, 3, 2], [2, 1]);
print(q);  // → [3, -1, 2]   (quotient: 3 - x + 2x²)
print(r);  // → [-1]         (remainder: -1)

// Evaluate 3 + x + 2x² at x = 4 using Horner's method
print(polynomialEvaluate([3, 1, 2], 4));  // → 39
```

## API

| Function | Returns | Notes |
|----------|---------|-------|
| `normalize(p)` | stripped copy | Removes trailing zeros |
| `polynomialDegree(p)` | `int` | Returns -1 for zero polynomial |
| `polynomialZero()` | `[]` | Additive identity |
| `polynomialOne()` | `[1]` | Multiplicative identity |
| `polynomialAdd(a, b)` | sum | Normalized |
| `polynomialSubtract(a, b)` | difference | Normalized |
| `polynomialMultiply(a, b)` | product | Normalized |
| `polynomialDivmod(a, b)` | `(quotient, remainder)` | Throws if b = zero |
| `polynomialDivide(a, b)` | quotient | Throws if b = zero |
| `polynomialMod(a, b)` | remainder | Throws if b = zero |
| `polynomialEvaluate(p, x)` | `num` | Horner's method |
| `polynomialGcd(a, b)` | GCD | Euclidean algorithm |

## How It Fits in the Stack

```
MA00  polynomial    ← THIS PACKAGE
MA01  gf256         ← GF(2^8) field arithmetic
MA02  reed-solomon  ← RS encoding/decoding, uses both MA00 and MA01
MA03  qr-encoder    ← QR code generation
```

## Running Tests

```bash
dart pub get
dart test
```

## Spec

[MA00-polynomial.md](../../../../specs/MA00-polynomial.md)
