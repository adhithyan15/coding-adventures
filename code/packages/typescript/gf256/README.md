# @coding-adventures/gf256

Galois Field GF(2^8) arithmetic. The field with 256 elements, used by
Reed-Solomon error correction, QR codes, and AES encryption.

## Stack Position

Layer MA01 — builds on MA00 polynomial, enables:
- **MA02 reed-solomon** — Error correction codes for QR codes and data storage

## Key Insight

In GF(256), **addition is XOR** and **subtraction equals addition**. Multiplication
uses precomputed log/antilog tables, making it O(1).

## Usage

```typescript
import {
  add, subtract, multiply, divide, power, inverse,
  LOG, ALOG, PRIMITIVE_POLYNOMIAL
} from "@coding-adventures/gf256";

// Addition is XOR
add(0x53, 0xCA);         // → 0x99

// Multiplication via log tables
multiply(0x53, 0xCA);    // → 0x01  (they are inverses!)

// Inverse: a × inverse(a) = 1
inverse(0x53);           // → 0xCA

// Generator g=2 has order 255
power(2, 255);           // → 1
```

## API

| Function | Description |
|----------|-------------|
| `add(a, b)` | a XOR b |
| `subtract(a, b)` | a XOR b (same as add) |
| `multiply(a, b)` | Product via log/antilog tables |
| `divide(a, b)` | Quotient; throws if b=0 |
| `power(base, exp)` | Exponentiation; g^255=1 |
| `inverse(a)` | Multiplicative inverse; throws if a=0 |
| `zero()` | Returns 0 |
| `one()` | Returns 1 |

## Constants

- `PRIMITIVE_POLYNOMIAL = 0x11D` — the irreducible polynomial x^8+x^4+x^3+x^2+1
- `LOG[256]` — discrete logarithm table (LOG[0] unused)
- `ALOG[255]` — antilogarithm table (ALOG[i] = 2^i mod p)
