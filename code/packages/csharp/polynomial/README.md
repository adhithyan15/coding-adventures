# CodingAdventures.Polynomial (C#)

Coefficient-array polynomial arithmetic over real numbers, implementing the
[MA00 polynomial specification](../../../specs/MA00-polynomial.md).

## What this package does

A **polynomial** is an expression like `3 + 2x + 5x²`. This library stores
polynomials as `double[]` arrays where the **array index equals the degree** of
that term (little-endian, lowest degree first):

```
[3, 2, 5]   →   3 + 2x + 5x²
[1]         →   1   (constant polynomial)
[]          →   0   (the zero polynomial)
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

Polynomial arithmetic is the foundation that GF(256) and Reed-Solomon build on.

## API

| Method | Description |
|--------|-------------|
| `Polynomial.Normalize(p)` | Strip trailing zero coefficients |
| `Polynomial.Degree(p)` | Highest non-zero index; −1 for the zero polynomial |
| `Polynomial.Zero()` | Return `[]` (additive identity) |
| `Polynomial.One()` | Return `[1]` (multiplicative identity) |
| `Polynomial.Add(a, b)` | Term-by-term addition |
| `Polynomial.Subtract(a, b)` | Term-by-term subtraction |
| `Polynomial.Multiply(a, b)` | Polynomial convolution |
| `Polynomial.DivMod(a, b)` | Long division: `(quotient, remainder)` |
| `Polynomial.Divide(a, b)` | Quotient only |
| `Polynomial.Mod(a, b)` | Remainder only |
| `Polynomial.Evaluate(p, x)` | Horner's method evaluation at a point |
| `Polynomial.Gcd(a, b)` | Euclidean GCD |
| `Polynomial.Format(p)` | Human-readable string (debug helper) |

## Quick example

```csharp
using CodingAdventures.Polynomial;

// (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
var a = new double[] { 1, 2 };
var b = new double[] { 3, 4 };
var product = Polynomial.Multiply(a, b);
// → [3, 10, 8]

// Evaluate 3 + x + 2x² at x = 4
var p = new double[] { 3, 1, 2 };
var value = Polynomial.Evaluate(p, 4);
// → 39.0

// Divide 5 + x + 3x² + 2x³  by  2 + x
var (q, r) = Polynomial.DivMod(new double[] { 5, 1, 3, 2 }, new double[] { 2, 1 });
// q → [3, -1, 2]   r → [-1]
```

## Running tests

```bash
dotnet test tests/CodingAdventures.Polynomial.Tests/CodingAdventures.Polynomial.Tests.csproj
```

## Version

0.1.0 — MA00 initial implementation.
