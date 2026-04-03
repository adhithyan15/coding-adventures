# @coding-adventures/polynomial

Polynomial arithmetic over real numbers. A polynomial is stored as a
coefficient array where the array index equals the degree:

```
[3, 0, 2]  →  3 + 0·x + 2·x²
```

## Stack Position

Layer MA00 — the foundation for:
- **MA01 gf256** — Galois Field GF(2^8) arithmetic (Reed-Solomon, QR codes, AES)
- **MA02 reed-solomon** — Error correction codes

## Usage

```typescript
import { add, multiply, evaluate, gcd } from "@coding-adventures/polynomial";

// 1 + 2x + 3x²
const p = [1, 2, 3];

// Evaluate at x = 2:  1 + 4 + 12 = 17
evaluate(p, 2); // → 17

// Multiply (1 + x)(2 + x) = 2 + 3x + x²
multiply([1, 1], [2, 1]); // → [2, 3, 1]

// GCD of two polynomials
gcd([2, 3, 1], [1, 1]); // → [2, 1] or scalar multiple
```

## API

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing zeros |
| `degree(p)` | Highest non-zero index; -1 for zero polynomial |
| `zero()` | Additive identity `[]` |
| `one()` | Multiplicative identity `[1]` |
| `add(a, b)` | Term-by-term addition |
| `subtract(a, b)` | Term-by-term subtraction |
| `multiply(a, b)` | Polynomial convolution |
| `divmod(a, b)` | Long division → `[quotient, remainder]` |
| `divide(a, b)` | Quotient only |
| `mod(a, b)` | Remainder only |
| `evaluate(p, x)` | Horner's method evaluation |
| `gcd(a, b)` | Euclidean GCD |

## Edge Cases

- Zero polynomial: `[]` (empty array)
- `degree([]) = -1` by convention
- `divmod(a, [])` throws — division by zero
- All results are normalized (trailing zeros stripped)
