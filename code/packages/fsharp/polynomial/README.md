# CodingAdventures.Polynomial (F#)

Coefficient-array polynomial arithmetic over real numbers, implementing the
[MA00 polynomial specification](../../../specs/MA00-polynomial.md).

## What this package does

A **polynomial** is an expression like `3 + 2x + 5x²`. This library stores
polynomials as `float array` values where the **array index equals the degree**
of that term (little-endian, lowest degree first):

```fsharp
[| 3.0; 2.0; 5.0 |]   →   3 + 2x + 5x²
[| 1.0 |]             →   1   (constant polynomial)
[||]                  →   0   (the zero polynomial)
```

All operations normalize their output — trailing zeros are always stripped.

## Where this fits

```
MA00  polynomial     ← this package
  ↓
MA01  gf256          (finite field arithmetic)
  ↓
MA02  reed-solomon   (error-correcting codes)
```

## API

| Function | Type | Description |
|----------|------|-------------|
| `Polynomial.normalize` | `Polynomial -> Polynomial` | Strip trailing zero coefficients |
| `Polynomial.degree` | `Polynomial -> int` | Highest non-zero index; −1 for zero polynomial |
| `Polynomial.zero` | `unit -> Polynomial` | Return `[||]` (additive identity) |
| `Polynomial.one` | `unit -> Polynomial` | Return `[|1|]` (multiplicative identity) |
| `Polynomial.add` | `Polynomial -> Polynomial -> Polynomial` | Term-by-term addition |
| `Polynomial.subtract` | `Polynomial -> Polynomial -> Polynomial` | Term-by-term subtraction |
| `Polynomial.multiply` | `Polynomial -> Polynomial -> Polynomial` | Polynomial convolution |
| `Polynomial.divmod` | `Polynomial -> Polynomial -> Polynomial * Polynomial` | Long division: `(quotient, remainder)` |
| `Polynomial.divide` | `Polynomial -> Polynomial -> Polynomial` | Quotient only |
| `Polynomial.pmod` | `Polynomial -> Polynomial -> Polynomial` | Remainder only |
| `Polynomial.evaluate` | `Polynomial -> float -> float` | Horner's method evaluation |
| `Polynomial.gcd` | `Polynomial -> Polynomial -> Polynomial` | Euclidean GCD |

Note: the remainder function is named `pmod` (polynomial mod) rather than `mod`
to avoid shadowing F#'s built-in `mod` operator.

## Quick example

```fsharp
open CodingAdventures.Polynomial

// (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
let a = [| 1.0; 2.0 |]
let b = [| 3.0; 4.0 |]
let product = Polynomial.multiply a b
// → [| 3.0; 10.0; 8.0 |]

// Evaluate 3 + x + 2x² at x = 4
let p = [| 3.0; 1.0; 2.0 |]
let value = Polynomial.evaluate p 4.0
// → 39.0

// Divide 5 + x + 3x² + 2x³  by  2 + x
let q, r = Polynomial.divmod [| 5.0; 1.0; 3.0; 2.0 |] [| 2.0; 1.0 |]
// q → [| 3.0; -1.0; 2.0 |]   r → [| -1.0 |]
```

## Running tests

```bash
dotnet test tests/CodingAdventures.Polynomial.Tests/CodingAdventures.Polynomial.Tests.fsproj
```

## Version

0.1.0 — MA00 initial implementation.
