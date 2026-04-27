# @coding-adventures/gf929

Galois Field GF(929) prime field arithmetic — the math behind PDF417 Reed-Solomon error correction.

## What is GF(929)?

GF(929) is a **prime field**: the integers modulo 929. Because 929 is prime, every non-zero element has a multiplicative inverse. This makes GF(929) a valid field for Reed-Solomon error correction.

This is fundamentally different from GF(256) (the binary extension field used by QR Code), which uses polynomial arithmetic over GF(2). In GF(929), arithmetic is ordinary modular integer arithmetic.

```
GF(929) = ℤ/929ℤ = { 0, 1, 2, ..., 928 }

add(a, b)  = (a + b)       mod 929
sub(a, b)  = (a - b + 929) mod 929
mul(a, b)  = (a * b)       mod 929
inv(b)     = b^{927}       mod 929   ← Fermat's little theorem
```

## Why this exists

PDF417 barcodes use 929 distinct codeword values (0–928). Reed-Solomon error correction requires the field size to match the alphabet size. Since 929 is prime, GF(929) exists and has exactly the right size.

The generator (primitive root) is α = 3, as specified in ISO/IEC 15438:2015, Annex A.4.

## Installation

This package is part of the coding-adventures monorepo and is referenced via `file:` dependencies.

## Usage

```typescript
import { add, subtract, multiply, divide, inverse, power, EXP, LOG } from "@coding-adventures/gf929";

// Basic arithmetic
add(100, 900)    // → 71  ((100 + 900) mod 929 = 1000 mod 929 = 71)
subtract(5, 10)  // → 924 ((5 - 10 + 929) mod 929 = 924)
multiply(3, 3)   // → 9
divide(9, 3)     // → 3

// Fermat's little theorem: a^{p-1} ≡ 1 mod p
power(3, 928)    // → 1

// Inverse: a × inverse(a) = 1
inverse(3)       // → 310  (3 × 310 = 930 ≡ 1 mod 929)

// Log/antilog tables for O(1) field operations
EXP[1]   // → 3    (α^1 = 3)
EXP[3]   // → 27   (α^3 = 3^3 = 27)
LOG[3]   // → 1    (discrete log base 3 of 3)
```

## API

| Function | Description |
|----------|-------------|
| `add(a, b)` | Addition in GF(929): (a + b) mod 929 |
| `subtract(a, b)` | Subtraction in GF(929): (a - b + 929) mod 929 |
| `multiply(a, b)` | Multiplication via log tables: O(1) |
| `divide(a, b)` | Division: a × b⁻¹ |
| `inverse(a)` | Multiplicative inverse: a^{927} mod 929 |
| `power(base, exp)` | Exponentiation via log tables |
| `EXP` | Antilogarithm table: EXP[i] = α^i mod 929 |
| `LOG` | Logarithm table: LOG[v] = discrete log base α of v |

## Dependency

This package has no dependencies — it is pure arithmetic.

## Where this fits

```
gf929 ← pdf417
```

The `pdf417` package imports `gf929` for Reed-Solomon over GF(929). The `gf929` package has no other dependents in this repo.
