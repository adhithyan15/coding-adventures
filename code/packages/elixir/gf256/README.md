# coding_adventures_gf256

**MA01** — Galois Field GF(2^8) arithmetic. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What It Does

Implements arithmetic in the finite field GF(256) — the field of 256 elements
used by Reed-Solomon error correction and AES encryption. Uses the primitive
polynomial `x^8 + x^4 + x^3 + x^2 + 1 = 0x11D`.

```elixir
alias CodingAdventures.GF256, as: GF

# Addition is XOR in characteristic-2
GF.add(0x53, 0xCA)           # => 0x99

# Multiplication via log/antilog tables
GF.multiply(0x53, 0x8C)      # => 1  (they are multiplicative inverses)

# Inverse: a * inverse(a) = 1
GF.inverse(0x53)             # => 0x8C
GF.multiply(5, GF.inverse(5))  # => 1

# Power
GF.power(2, 255)             # => 1  (generator has order 255)
GF.power(2, 8)               # => 29  (0x1D, after first modular reduction)

# Division
GF.divide(10, 5)             # some non-zero field element
```

## Where It Fits

```
MA02 Reed-Solomon
  └── MA01 GF(256)  ← this package
        └── MA00 Polynomial (conceptually, not a code dependency)
```

## API

| Function | Description |
|---|---|
| `add(a, b)` | a XOR b |
| `subtract(a, b)` | a XOR b (same as add in char-2) |
| `multiply(a, b)` | GF multiplication via log tables |
| `divide(a, b)` | GF division — raises `ArgumentError` if b = 0 |
| `power(base, exp)` | base^exp in GF(256) |
| `inverse(a)` | multiplicative inverse — raises `ArgumentError` if a = 0 |
| `zero()` | additive identity 0 |
| `one()` | multiplicative identity 1 |
| `alog_table()` | antilog table for inspection/testing |
| `log_table()` | log table for inspection/testing |

## Running Tests

```bash
mix test
```

## Constants

- `@primitive_polynomial 0x11D` — the Reed-Solomon primitive polynomial
- Tables are built at compile time — no runtime overhead

## Version

0.1.0 — MA01 spec compliant.
