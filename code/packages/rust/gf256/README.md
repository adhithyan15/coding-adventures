# gf256

Galois Field GF(2^8) arithmetic — the math behind Reed-Solomon error correction.

## Overview

GF(2^8) is a finite field with 256 elements (the integers 0–255). It is used by:

- **Reed-Solomon error correction** — QR codes, CDs, DVDs, hard drives
- **AES encryption** — SubBytes and MixColumns steps
- **CRCs and checksums** — polynomial division over GF(2)

The key insight: in GF(2^8), **addition IS XOR** and **subtraction IS addition**.
No carry. No overflow. Just XOR.

Multiplication uses precomputed log/antilog tables (initialized once via
`std::sync::OnceLock`) to achieve O(1) performance.

## Primitive Polynomial

```text
p(x) = x^8 + x^4 + x^3 + x^2 + 1   =   0x11D   =   285
```

This is the standard primitive polynomial for most Reed-Solomon implementations.
Note: AES uses 0x11B — a different polynomial.

## Where This Fits

```
MA00  polynomial      — polynomial arithmetic over f64
MA01  gf256           ← this crate
MA02  reed-solomon    — Reed-Solomon error correction
```

## Usage

```rust
use gf256::{add, subtract, multiply, divide, power, inverse, ZERO, ONE};

// Addition is XOR
assert_eq!(add(0x53, 0xca), 0x99);

// 0x53 and 0xCA are multiplicative inverses under 0x11D
assert_eq!(multiply(0x53, 0xca), 1);
assert_eq!(inverse(0x53), 0xca);

// Generator 2 has order 255
assert_eq!(power(2, 255), 1);

// 2^8 = 0x1D (first reduction step)
assert_eq!(power(2, 8), 0x1d);

// divide
assert_eq!(divide(multiply(3, 5), 5), 3);
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `add(a, b)` | `(u8, u8) → u8` | XOR |
| `subtract(a, b)` | `(u8, u8) → u8` | XOR (same as add) |
| `multiply(a, b)` | `(u8, u8) → u8` | log-table multiplication |
| `divide(a, b)` | `(u8, u8) → u8` | division; panics if b = 0 |
| `power(base, exp)` | `(u8, u32) → u8` | exponentiation |
| `inverse(a)` | `(u8) → u8` | multiplicative inverse; panics if a = 0 |

Constants: `ZERO = 0`, `ONE = 1`, `PRIMITIVE_POLYNOMIAL = 0x11D`.

## Spec

See `code/specs/MA01-gf256.md` in the coding-adventures monorepo.
