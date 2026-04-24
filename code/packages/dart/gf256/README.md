# coding_adventures_gf256

GF(256) Galois Field arithmetic for Reed-Solomon error correction.

## What Is GF(256)?

GF(256) — Galois Field of 2^8 — is a finite field with exactly 256 elements
(the integers 0..255). Unlike ordinary integer arithmetic, GF(256) arithmetic
is closed under all operations and every non-zero element has a multiplicative
inverse.

Three important systems rely on GF(256):

| System | Role of GF(256) |
|--------|----------------|
| **Reed-Solomon codes** | QR codes, CDs, DVDs — bytes are field elements; RS is polynomial arithmetic over this field |
| **AES encryption** | SubBytes (S-box) and MixColumns steps use GF(2^8) |
| **RAID-6** | The two parity drives are an RS code over GF(256) |

This package uses the **Reed-Solomon primitive polynomial**
`p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D`, which is the standard for
QR codes and most RS implementations (AES uses 0x11B).

## Key Insight: Addition Is XOR

In GF(2^8), `1 + 1 = 0` (characteristic 2). This means:

- Every element is its own additive inverse: `a + a = 0`
- Subtraction equals addition: `a - b = a + b = a XOR b`
- No carry, no overflow — XOR is exact

## Usage

```dart
import 'package:coding_adventures_gf256/coding_adventures_gf256.dart';

// Addition is XOR
print(gfAdd(0x53, 0xCA));   // → 153 (0x99)

// Every element is its own inverse
print(gfAdd(42, 42));       // → 0

// Multiplication via log/antilog tables (O(1))
print(gfMultiply(2, 128));  // → 29 (reduction modulo 0x11D)

// Multiplicative inverse: a × inverse(a) = 1
print(gfMultiply(0x53, gfInverse(0x53)));  // → 1

// Power: 2^255 = 1 (cyclic group of order 255)
print(gfPower(2, 255));  // → 1
```

## How It Fits in the Stack

```
MA00  polynomial    ← coefficient-array polynomial arithmetic
MA01  gf256         ← THIS PACKAGE (GF(2^8) field arithmetic)
MA02  reed-solomon  ← RS encoding/decoding, uses both MA00 and MA01
MA03  qr-encoder    ← QR code generation, uses MA02 for error correction
```

## API

| Function | Returns | Notes |
|----------|---------|-------|
| `gfAdd(a, b)` | `a ^ b` | XOR; no tables needed |
| `gfSubtract(a, b)` | `a ^ b` | Same as add in characteristic 2 |
| `gfMultiply(a, b)` | product | Uses log/antilog tables |
| `gfDivide(a, b)` | quotient | Throws if b = 0 |
| `gfPower(base, exp)` | base^exp | Throws if exp < 0 |
| `gfInverse(a)` | a^(-1) | Throws if a = 0 |
| `gfZero()` | 0 | Additive identity |
| `gfOne()` | 1 | Multiplicative identity |
| `alog` | `List<int>` | Antilogarithm table (lazy-built) |
| `log` | `List<int>` | Logarithm table (lazy-built) |

## Running Tests

```bash
dart pub get
dart test
```

## Spec

[MA01-gf256.md](../../../../specs/MA01-gf256.md)
