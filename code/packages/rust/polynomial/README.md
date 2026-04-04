# polynomial

Polynomial arithmetic over `f64` coefficients — add, subtract, multiply,
divide, evaluate, and compute the GCD of two polynomials.

## Overview

A polynomial is stored as a `Vec<f64>` (or `&[f64]` slice) where the **array
index equals the degree** of that term's coefficient ("little-endian" order):

```text
[3.0, 0.0, 2.0]   →   3 + 0·x + 2·x²   =   3 + 2x²
[1.0, 2.0, 3.0]   →   1 + 2x + 3x²
[]                 →   the zero polynomial
```

All returned polynomials are **normalized** — trailing near-zero coefficients are
stripped so that `[1.0, 0.0, 0.0]` and `[1.0]` both represent the same constant `1`.

## Where This Fits

This crate is layer **MA00** in the coding-adventures math stack:

```
MA00  polynomial      ← this crate
MA01  gf256           — Galois Field GF(2^8) arithmetic
MA02  reed-solomon    — Reed-Solomon error correction
```

## Usage

```rust
use polynomial::{add, subtract, multiply, divmod, evaluate, gcd, normalize};

// 3 + 2x²
let p = vec![3.0, 0.0, 2.0];

// Evaluate at x = 2: 3 + 0·2 + 2·4 = 11
let val = evaluate(&p, 2.0);
assert!((val - 11.0).abs() < 1e-12);

// (x² - 1) / (x - 1) = (x + 1), remainder 0
let dividend = vec![-1.0, 0.0, 1.0];  // x² - 1
let divisor  = vec![-1.0, 1.0];       // x - 1
let (quotient, remainder) = divmod(&dividend, &divisor);
// quotient ≈ [1.0, 1.0]  (x + 1)
// remainder ≈ []          (zero)

// GCD of (x-1)(x-2) and (x-1)
let a = vec![2.0, -3.0, 1.0];  // x² - 3x + 2
let b = vec![-1.0, 1.0];       // x - 1
let g = gcd(&a, &b);
// g is proportional to [-1.0, 1.0]  (x - 1)
```

## API

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing near-zero coefficients |
| `degree(p)` | Degree of the polynomial (0 for zero poly) |
| `zero()` | Returns `[0.0]` |
| `one()` | Returns `[1.0]` |
| `add(a, b)` | Polynomial addition |
| `subtract(a, b)` | Polynomial subtraction |
| `multiply(a, b)` | Polynomial multiplication (convolution) |
| `divmod(a, b)` | Long division → `(quotient, remainder)` |
| `divide(a, b)` | Quotient only |
| `modulo(a, b)` | Remainder only |
| `evaluate(p, x)` | Horner's-method evaluation at `x` |
| `gcd(a, b)` | Euclidean GCD |

## Spec

See `code/specs/MA00-polynomial.md` in the coding-adventures monorepo.
