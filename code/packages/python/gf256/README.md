# coding-adventures-gf256

Galois Field GF(2^8) arithmetic. The 256-element field used by Reed-Solomon
error correction, QR codes, and AES encryption.

## Stack Position

Layer MA01 — enables:
- **MA02 reed-solomon** — Error correction codes for QR codes and data storage

## Key Insight

In GF(256), **addition is XOR** and **subtraction equals addition**. Multiplication
uses precomputed log/antilog tables for O(1) performance.

## Usage

```python
from gf256 import add, multiply, inverse, power, LOG, ALOG

add(0x53, 0xCA)      # → 0x99 (XOR)
multiply(0x53, 0xCA) # → 1    (they are inverses!)
inverse(0x53)        # → 0xCA
power(2, 255)        # → 1    (g^255 = 1)
```

## API

| Function | Description |
|----------|-------------|
| `add(a, b)` | a XOR b |
| `subtract(a, b)` | a XOR b (same as add) |
| `multiply(a, b)` | Product via log/antilog tables |
| `divide(a, b)` | Quotient; raises ValueError if b=0 |
| `power(base, exp)` | Exponentiation; g^255=1 |
| `inverse(a)` | Multiplicative inverse; raises ValueError if a=0 |
| `zero()` | Returns 0 |
| `one()` | Returns 1 |

## Constants

- `PRIMITIVE_POLYNOMIAL = 0x11D`
- `LOG` — 256-element discrete logarithm table
- `ALOG` — 255-element antilogarithm table
