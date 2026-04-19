# coding-adventures-polynomial

Univariate polynomial arithmetic. A polynomial is stored as a tuple
where the index equals the degree:

```python
(3, 0, 2)  →  3 + 0·x + 2·x²
```

The package is coefficient-agnostic — any numeric type that supports
`+ - * /` works. In practice the callers are:

- **`gf256` / `reed-solomon`** — integer coefficients over GF(2^8).
- **Symbolic integration (CAS)** — `fractions.Fraction` coefficients
  over Q[x] for exact rational arithmetic.

## Stack Position

Layer MA00 — the foundation for:

- **MA01 gf256** — Galois Field GF(2^8) arithmetic
- **MA02 reed-solomon** — Error correction codes
- **CAS Phase 2** — Hermite reduction and Rothstein–Trager in
  `symbolic-vm`'s Risch-track integrator

## Usage

### Integer / real coefficients

```python
from polynomial import add, multiply, evaluate, gcd, divmod_poly

p = (1, 2, 3)                     # 1 + 2x + 3x²

evaluate(p, 2)                    # → 17   (1 + 4 + 12)
multiply((1, 1), (2, 1))          # → (2, 3, 1)   (x+1)(x+2)

q, r = divmod_poly((5, 1, 3, 2), (2, 1))
# q = (3, -1, 2), r = (-1,)
```

### Exact rationals — the CAS path

```python
from fractions import Fraction
from polynomial import deriv, monic, squarefree, multiply

x1 = (Fraction(-1), Fraction(1))  # x - 1
x2 = (Fraction(-2), Fraction(1))  # x - 2
x3 = (Fraction(-3), Fraction(1))  # x - 3

# p = (x-1)·(x-2)²·(x-3)³
p = multiply(x1, multiply(multiply(x2, x2), multiply(x3, multiply(x3, x3))))

deriv(p)          # formal derivative — still exact Fractions
monic(p)          # rescale so leading coefficient is 1
squarefree(p)     # → [x-1, x-2, x-3]  (multiplicities encoded by position)
```

## API

### Core

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing zeros |
| `degree(p)` | Highest non-zero index; `-1` for zero polynomial |
| `zero()` | Additive identity `()` |
| `one()` | Multiplicative identity `(1,)` |
| `add(a, b)` | Term-by-term addition |
| `subtract(a, b)` | Term-by-term subtraction |
| `multiply(a, b)` | Polynomial convolution |
| `divmod_poly(a, b)` | Long division → `(quotient, remainder)` |
| `divide(a, b)` | Quotient only |
| `mod(a, b)` | Remainder only |
| `evaluate(p, x)` | Horner's method; return type matches coefficient ring |
| `gcd(a, b)` | Euclidean GCD |

### Calculus and factorization

| Function | Description |
|----------|-------------|
| `deriv(p)` | Formal derivative `d/dx` |
| `monic(p)` | Rescale leading coefficient to 1 (needs a field) |
| `squarefree(p)` | Yun's algorithm: `p = c · s_1 · s_2² · … · s_k^k` with each `s_i` monic, squarefree, pairwise coprime |

## Edge Cases

- Zero polynomial: `()` (empty tuple)
- `degree(()) == -1` by convention
- `divmod_poly(a, ())` raises `ValueError`
- All results are normalized (trailing zeros stripped)
- `squarefree(())` and `squarefree((c,))` both return `[]` — no
  squarefree factors to record, the constant is absorbed as the
  implicit prefactor
