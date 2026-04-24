# gf256 — Kotlin

Galois Field GF(2^8) arithmetic over the Reed-Solomon primitive polynomial
`x^8 + x^4 + x^3 + x^2 + 1` (= `0x11D`).

## What is GF(256)?

GF(256) is a finite field with exactly 256 elements (the integers 0..255).
Its arithmetic powers:

- **Reed-Solomon error correction** — QR codes, CDs, DVDs, RAID-6
- **AES encryption** — SubBytes (S-box) and MixColumns steps

## Key insight: Add = XOR = Subtract

In a characteristic-2 field, 1 + 1 = 0. Every element is its own additive
inverse. So addition and subtraction are both bitwise XOR — no carry, no
overflow.

## Multiplication via log tables

The element `g = 2` generates all 255 non-zero elements. We precompute:

```
EXP[i] = 2^i mod p(x)
LOG[x] = i such that 2^i = x
```

Then `a × b = EXP[LOG[a] + LOG[b]]` — O(1) with two table lookups.

## Usage

```kotlin
import com.codingadventures.gf256.GF256

val product = GF256.mul(0x53, 0x8C)   // → 1   (inverse pair under 0x11D)
val sum     = GF256.add(0x53, 0xCA)   // → 0x99
val inverse = GF256.inv(0x53)          // → 0x8C
```

## API

| Function | Description |
|----------|-------------|
| `add(a, b)` | XOR (addition = subtraction in GF(2^n)) |
| `sub(a, b)` | XOR (same as add) |
| `mul(a, b)` | Multiplication via log/exp tables |
| `div(a, b)` | Division; throws `ArithmeticException` when b = 0 |
| `pow(base, n)` | Exponentiation; 0^0 = 1 by convention |
| `inv(a)` | Multiplicative inverse; throws when a = 0 |

Constants: `PRIMITIVE_POLY = 0x11D`, `EXP[512]`, `LOG[256]`

## Build

```
gradle test
```

Requires JDK 21 on PATH. Uses JUnit Jupiter 5.11.4.

## Spec

`code/specs/MA01-gf256.md`
