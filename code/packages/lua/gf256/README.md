# coding-adventures-gf256 (Lua)

Galois Field GF(2^8) arithmetic — part of the `coding-adventures` math stack
(layer MA01). Used by Reed-Solomon error correction (MA02) and QR code generation.

## What It Does

GF(256) is a finite field with 256 elements (0–255). Arithmetic is radically
different from ordinary integer arithmetic:

- **Addition = XOR** (no carry, no overflow — ever)
- **Subtraction = XOR** (same as addition in characteristic 2)
- **Multiplication** via precomputed log/antilog tables (O(1) two lookups)
- **Division** via log/antilog tables
- **Every non-zero element** has a multiplicative inverse

The primitive polynomial used is:

```
p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
```

This is the Reed-Solomon / QR-code standard (not the AES polynomial 0x11B).

## Stack Position

```
MA03 — qr-encoder     (uses reed-solomon)
MA02 — reed-solomon   (uses polynomial + gf256)
MA01 — gf256          ← you are here
MA00 — polynomial     (conceptual foundation)
```

## Quick Start

```lua
local gf = require("coding_adventures.gf256")

-- Addition is XOR
print(gf.add(0x53, 0xCA))      -- 0x99

-- Every element is its own additive inverse
print(gf.add(0x53, 0x53))      -- 0  (characteristic 2: x + x = 0)

-- Multiplication via log/antilog tables
print(gf.multiply(0x53, 0x8C)) -- 1  (they are multiplicative inverses)

-- Inverse
print(gf.inverse(0x53))        -- 0x8C

-- Generator g=2 has order 255
print(gf.power(2, 255))        -- 1

-- Division
print(gf.divide(0x53, 0x8C))   -- 1  (= multiply(0x53, inverse(0x8C)))
```

## API Reference

| Function | Description |
|----------|-------------|
| `add(a, b)` | a XOR b |
| `subtract(a, b)` | a XOR b (same as add in GF(2^8)) |
| `multiply(a, b)` | GF(256) product via log/antilog tables |
| `divide(a, b)` | GF(256) quotient; errors if b = 0 |
| `power(base, exp)` | base^exp in GF(256) |
| `inverse(a)` | Multiplicative inverse; errors if a = 0 |

**Constants:**

| Name | Value | Meaning |
|------|-------|---------|
| `ZERO` | 0 | Additive identity |
| `ONE` | 1 | Multiplicative identity |
| `PRIMITIVE_POLYNOMIAL` | 0x11D | Irreducible polynomial for modular reduction |

## How the Tables Work

At module load time, the package builds two lookup tables:

- `ALOG[i]` = g^i mod p(x) (antilogarithm; exponent → field element)
- `LOG[x]`  = i such that g^i = x (logarithm; field element → exponent)

Then multiplication becomes:

```
a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
```

Two table lookups and one addition — no bit manipulation needed.

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

## Dependencies

- Lua ≥ 5.4 (uses `~` for XOR, `<<` for bit shift)
- No external Lua dependencies

## License

MIT
