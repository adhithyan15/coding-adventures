# polynomial — Kotlin

Polynomial arithmetic over GF(2^8) (Galois Field with 256 elements).

## What are GF(256) polynomials?

These are polynomials whose coefficients are GF(256) elements (bytes 0..255).
All arithmetic — addition, subtraction, multiplication, division — uses GF(256)
field operations rather than ordinary integer math.

This is the mathematical layer between raw byte arithmetic (gf256) and
Reed-Solomon error correction (reed-solomon).

## Representation

Polynomials are little-endian `IntArray`s: index `i` holds the coefficient
of `x^i`.

```
[3, 0, 2]  →  3 + 0·x + 2·x²
[1]        →  constant 1
[]         →  zero polynomial
```

## Key differences from real-number polynomials

| Operation     | Real polynomials | GF(256) polynomials       |
|---------------|-----------------|---------------------------|
| Add coeff     | a + b           | a XOR b                   |
| Sub coeff     | a − b           | a XOR b  (same as add!)   |
| Mul coeff     | a × b           | `GF256.mul(a, b)`         |
| Div coeff     | a / b           | `GF256.div(a, b)`         |

## Usage

```kotlin
import com.codingadventures.polynomial.*

val p = poly(3, 0, 2)       // 3 + 2x²
val q = poly(1, 1)          // 1 + x

val sum  = add(p, q)        // coefficient-wise XOR
val prod = mul(p, q)        // polynomial convolution in GF(256)
val (quotient, rem) = divmod(prod, q)

val root = eval(prod, 4)    // Horner evaluation at x=4 in GF(256)
```

## API

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing zero coefficients |
| `degree(p)` | Degree of p; -1 for zero polynomial |
| `zero()` | The zero polynomial `[]` |
| `one()` | The identity polynomial `[1]` |
| `poly(vararg)` | Construct and normalise a polynomial |
| `add(a, b)` | XOR each coefficient |
| `sub(a, b)` | Same as add in GF(256) |
| `mul(a, b)` | Polynomial convolution using GF(256) mul |
| `divmod(a, b)` | Long division → (quotient, remainder) |
| `divide(a, b)` | Quotient of divmod |
| `mod(a, b)` | Remainder of divmod |
| `eval(p, x)` | Horner evaluation at GF(256) point x |
| `gcd(a, b)` | GCD via Euclidean algorithm |

## Dependencies

- `gf256` — GF(2^8) field operations (resolved as a local composite build)

## Build

```
gradle test
```

## Spec

`code/specs/MA00-polynomial.md`
