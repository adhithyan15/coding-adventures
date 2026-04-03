# go/polynomial

Polynomial arithmetic over float64 coefficients. Polynomials are represented
as `[]float64` where index i is the coefficient of x^i.

## Stack Position

Layer MA00 — enables MA01 (gf256) and MA02 (reed-solomon).

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/polynomial"

// 1 + 2x + 3x²
p := []float64{1, 2, 3}
polynomial.Evaluate(p, 2)  // → 17.0

// (1+x)(2+x) = 2 + 3x + x²
polynomial.Multiply([]float64{1,1}, []float64{2,1})

// Long division
q, r := polynomial.Divmod([]float64{5,1,3,2}, []float64{2,1})
// q=[3,-1,2], r=[-1]
```

## API

- `Normalize(p)`, `Degree(p)`, `Zero()`, `One()`
- `Add(a, b)`, `Subtract(a, b)`, `Multiply(a, b)`
- `Divmod(a, b)` — panics for zero divisor
- `Divide(a, b)`, `Mod(a, b)`
- `Evaluate(p, x)` — Horner's method
- `GCD(a, b)` — Euclidean algorithm
