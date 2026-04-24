# polynomial (Haskell)

Coefficient-array polynomial arithmetic over GF(256).

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
math foundation, implementing **MA00** from the spec series.

## What Is This?

A polynomial is a mathematical expression like `3 + 2x + x²`. This package
represents polynomials as coefficient arrays in **little-endian order**
(index = degree):

```haskell
Poly [3, 2, 1]  -- 3 + 2x + x²
Poly [1]        -- constant 1
Poly []         -- zero polynomial
```

All coefficient arithmetic uses **GF(256) operations** from the `gf256` package:

- Addition/subtraction = XOR
- Multiplication = log/antilog table lookup
- Division = log/antilog table lookup

## Operations

| Function | Description |
|----------|-------------|
| `polyAdd` | Term-by-term addition (XOR) |
| `polySub` | Term-by-term subtraction (= addition in GF(256)) |
| `polyMul` | Polynomial convolution |
| `polyDivMod` | Long division, returns `(quotient, remainder)` |
| `polyDiv` | Long division quotient |
| `polyMod` | Long division remainder |
| `polyEval` | Evaluate at a GF(256) point (Horner's method) |
| `polyGcd` | Euclidean GCD algorithm |
| `polyScale` | Multiply all coefficients by a scalar |
| `polyNormalize` | Strip trailing zeros |
| `polyDegree` | Highest non-zero index (−1 for zero polynomial) |

## Usage

```haskell
import Polynomial
import GF256 (gfMul)

-- Reed-Solomon generator polynomial for nCheck=2: (x+2)(x+4)
let f1  = Poly [2, 1]   -- (x + 2)
    f2  = Poly [4, 1]   -- (x + 4)
    gen = polyMul f1 f2 -- Poly [8, 6, 1]  =  8 + 6x + x²

-- Verify: generator evaluates to 0 at alpha^1 = 2
polyEval gen 2  -- = 0  (alpha^1 is a root)
polyEval gen 4  -- = 0  (alpha^2 is a root)

-- Long division
let (q, r) = polyDivMod (Poly [1,2,3,4]) (Poly [2,1])
-- a = b*q + r  (verified by polyAdd (polyMul b q) r == a)
```

## Package Structure

```
polynomial/
├── src/
│   └── Polynomial.hs     — implementation
├── test/
│   ├── Spec.hs            — test entry point
│   └── PolynomialSpec.hs  — Hspec tests
├── polynomial.cabal
├── BUILD
└── README.md
```

## Building and Testing

```bash
cabal test
```

## Dependencies

- **`gf256`**: All coefficient arithmetic delegates to GF(256) operations.
- **Upstream (MA02)**: The `reed-solomon` package depends on this package
  for polynomial operations over GF(256).

## Spec

See [`code/specs/MA00-polynomial.md`](../../../specs/MA00-polynomial.md)
for the full specification including algorithm diagrams and cross-language
test vectors.
