# coding-adventures-polynomial

Polynomial arithmetic over real numbers. A polynomial is stored as a
tuple where the index equals the degree:

```python
(3, 0, 2)  →  3 + 0·x + 2·x²
```

## Stack Position

Layer MA00 — the foundation for:
- **MA01 gf256** — Galois Field GF(2^8) arithmetic
- **MA02 reed-solomon** — Error correction codes

## Usage

```python
from polynomial import add, multiply, evaluate, gcd, divmod_poly

p = (1, 2, 3)  # 1 + 2x + 3x²

evaluate(p, 2)           # → 17.0  (1 + 4 + 12)
multiply((1, 1), (2, 1)) # → (2, 3, 1)  (x+1)(x+2)

q, r = divmod_poly((5, 1, 3, 2), (2, 1))
# q = (3, -1, 2), r = (-1,)
```

## API

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing zeros |
| `degree(p)` | Highest non-zero index; -1 for zero polynomial |
| `zero()` | Additive identity `()` |
| `one()` | Multiplicative identity `(1,)` |
| `add(a, b)` | Term-by-term addition |
| `subtract(a, b)` | Term-by-term subtraction |
| `multiply(a, b)` | Polynomial convolution |
| `divmod_poly(a, b)` | Long division → `(quotient, remainder)` |
| `divide(a, b)` | Quotient only |
| `mod(a, b)` | Remainder only |
| `evaluate(p, x)` | Horner's method evaluation |
| `gcd(a, b)` | Euclidean GCD |

## Edge Cases

- Zero polynomial: `()` (empty tuple)
- `degree(()) = -1` by convention
- `divmod_poly(a, ())` raises `ValueError`
- All results are normalized (trailing zeros stripped)
