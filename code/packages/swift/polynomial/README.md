# polynomial

Polynomial arithmetic over real numbers (Double) for Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer MA00.

## What It Does

This library implements the fundamental operations of polynomial algebra:
addition, subtraction, multiplication, long division, evaluation (Horner's
method), and GCD (Euclidean algorithm). It is the foundation for `gf256`
(Galois field arithmetic over GF(2^8)), which in turn underpins Reed-Solomon
error correction and AES encryption.

## Representation

A polynomial is a `[Double]` in ascending-degree ("little-endian") order:
index `i` holds the coefficient of `x^i`.

```
[3.0, 0.0, 2.0]  →  3 + 0·x + 2·x²  =  3 + 2x²
[1.0, 2.0, 3.0]  →  1 + 2x + 3x²
[0.0]            →  the zero polynomial
```

All functions return **normalized** polynomials — trailing near-zero
coefficients (below threshold 1e-10) are stripped. The zero polynomial
is always `[0.0]`, never an empty array.

## API

All functions live in the `Polynomial` enum namespace:

```swift
import Polynomial

// Constants
Polynomial.zero()                         // [0.0]
Polynomial.one()                          // [1.0]

// Utilities
Polynomial.normalize([1.0, 0.0, 0.0])    // [1.0]
Polynomial.degree([1.0, 2.0, 3.0])       // 2

// Arithmetic
Polynomial.add([1.0, 2.0], [3.0, 4.0])        // [4.0, 6.0]
Polynomial.subtract([5.0, 7.0], [1.0, 2.0])   // [4.0, 5.0]
Polynomial.multiply([1.0, 2.0], [3.0, 4.0])   // [3.0, 10.0, 8.0]

// Division
let (q, r) = Polynomial.divmod([-1.0, 0.0, 1.0], [-1.0, 1.0])
// q = [1.0, 1.0]  (x + 1),  r = [0.0]

Polynomial.divide(a, b)   // quotient of divmod(a, b)
Polynomial.mod(a, b)      // remainder of divmod(a, b)

// Evaluation (Horner's method)
Polynomial.evaluate([3.0, 1.0, 2.0], 4.0)  // 39.0

// GCD (Euclidean algorithm, monic result)
Polynomial.gcd([-1.0, 0.0, 1.0], [-1.0, 1.0])  // [-1.0, 1.0] (x - 1)
```

## Where It Fits

```
MA00 polynomial  ← you are here
      ↓
MA01 gf256       (GF(2^8) arithmetic using polynomial mod)
      ↓
Reed-Solomon, QR codes, AES, ...
```

## Running Tests

```bash
swift test
```

## License

MIT
