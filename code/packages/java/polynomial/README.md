# polynomial — Java

Polynomial arithmetic over an abstract coefficient field for Java.

## What This Is

A polynomial is stored as an `int[]` where the index equals the degree:

```
[3, 0, 2]  →  3 + 0·x + 2·x²
```

This "little-endian" convention makes addition trivially position-aligned.

All operations normalize their result — trailing zero coefficients are stripped.

## Field-agnostic Design

Arithmetic is parameterized by a `FieldOps` instance:

- `FieldOps.INTEGER_OPS` — ordinary integer arithmetic
- `FieldOps.GF256_OPS` — GF(2^8) Galois Field arithmetic (XOR for add)

This means the same `divmod`, `gcd`, and `evaluate` algorithms work for
tutorial examples over the integers and for actual Reed-Solomon arithmetic over GF(256).

## Usage

```java
import com.codingadventures.polynomial.Polynomial;
import com.codingadventures.polynomial.FieldOps;

// Integer arithmetic
int[] a = {1, 2};   // 1 + 2x
int[] b = {3, 4};   // 3 + 4x
int[] prod = Polynomial.mul(a, b, FieldOps.INTEGER_OPS);  // [3, 10, 8]

// GF(256) arithmetic (for Reed-Solomon)
int[] gen = Polynomial.mul(new int[]{2, 1}, new int[]{4, 1}, FieldOps.GF256_OPS);
// gen = [8, 6, 1]  (the RS generator for nCheck=2)

// Evaluate: Horner's method
int val = Polynomial.evaluate(gen, 2, FieldOps.GF256_OPS);  // = 0 (root at α=2)

// Long division
int[][] qr = Polynomial.divmod(a, b, FieldOps.INTEGER_OPS);  // [quotient, remainder]
```

## Spec

See `code/specs/MA00-polynomial.md` for the full specification.

## Tests

```
gradle test
```

Part of the [MA series](../../../../specs/MA00-polynomial.md) — the math foundation
for 2D barcodes.
