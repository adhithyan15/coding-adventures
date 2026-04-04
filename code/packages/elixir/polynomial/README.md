# coding_adventures_polynomial

**MA00** — Polynomial arithmetic over real numbers. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What It Does

Represents polynomials as coefficient lists (index = degree) and provides
the full arithmetic needed by higher layers: GF(2^8), Reed-Solomon codes, and CRCs.

```elixir
alias CodingAdventures.Polynomial, as: Poly

# 3 + 0x + x² = [3.0, 0.0, 1.0]
Poly.evaluate([3.0, 0.0, 1.0], 2.0)  # => 7.0

Poly.add([1.0, 2.0, 3.0], [4.0, 5.0])  # => [5.0, 7.0, 3.0]

Poly.multiply([1.0, 2.0], [3.0, 4.0])  # => [3.0, 10.0, 8.0]

{q, r} = Poly.divmod_poly([-1.0, 0.0, 1.0], [-1.0, 1.0])
# q = [1.0, 1.0]  (x+1)
# r = [0.0]       (no remainder)
```

## Where It Fits

```
MA02 Reed-Solomon
  └── MA01 GF(256)
        └── MA00 Polynomial  ← this package
```

## API

| Function | Description |
|---|---|
| `normalize(p)` | Strip trailing near-zero coefficients |
| `degree(p)` | Degree of polynomial |
| `zero()` | Additive identity `[0.0]` |
| `one()` | Multiplicative identity `[1.0]` |
| `add(a, b)` | Add two polynomials |
| `subtract(a, b)` | Subtract b from a |
| `multiply(a, b)` | Polynomial convolution |
| `divmod_poly(a, b)` | `{quotient, remainder}` — raises `ArgumentError` on zero b |
| `divide(a, b)` | Quotient of divmod |
| `modulo(a, b)` | Remainder of divmod |
| `evaluate(p, x)` | Horner's method evaluation |
| `gcd(a, b)` | Euclidean GCD, monic result |

## Running Tests

```bash
mix test
```

## Version

0.1.0 — MA00 spec compliant.
