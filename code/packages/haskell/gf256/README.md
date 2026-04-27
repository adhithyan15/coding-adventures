# gf256 (Haskell)

Galois Field GF(2^8) arithmetic for Reed-Solomon error correction.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
math foundation, implementing **MA01** from the spec series.

## What Is GF(256)?

GF(256) is a finite field with exactly 256 elements (0–255). The arithmetic
is very different from ordinary integers:

- **Addition = XOR**: In characteristic 2, `1 + 1 = 0`, so every element is
  its own additive inverse. Subtraction equals addition.
- **Multiplication = table lookup**: Uses log/antilog tables built from the
  primitive polynomial `p(x) = x^8 + x^4 + x^3 + x^2 + 1` (= `0x11D`).

All field operations are **O(1)** — two array accesses and an integer addition.

## Where Is It Used?

| System | How GF(256) Helps |
|--------|--------------------|
| QR codes | Reed-Solomon check bytes are GF(256) polynomial remainders |
| CDs / DVDs | Two-level CIRC error correction over GF(256) |
| Hard drives | Sector-level error correction firmware |
| AES encryption | MixColumns and SubBytes steps (different polynomial) |
| RAID-6 | Parity drives are a Reed-Solomon code over GF(256) |

## Usage

```haskell
import GF256

-- Addition is XOR
gfAdd 0x53 0xCA   -- = 0x99

-- Multiplication via log/antilog tables
gfMul 2 4         -- = 8
gfMul 2 128       -- = 29  (overflow reduction)

-- Inverse: a × gfInv a = 1
gfInv 0x53        -- = 0x8C  (under the 0x11D polynomial)
gfMul 0x53 0x8C   -- = 1

-- Power
gfPow 2 255       -- = 1  (g^255 = 1, cyclic group)

-- Division
gfDiv 8 2         -- = 4

-- Tables
expTable          -- Array Int Int, indices 0..255
logTable          -- Array Int Int, indices 0..255
```

## Package Structure

```
gf256/
├── src/
│   └── GF256.hs          — field implementation
├── test/
│   ├── Spec.hs            — test entry point
│   └── GF256Spec.hs       — Hspec tests
├── gf256.cabal
├── BUILD                  — monorepo build script
└── README.md
```

## Building and Testing

```bash
cabal test
```

## Dependencies

- **In the coding-adventures stack**: No local dependencies. This is the
  foundational field arithmetic package.
- **Upstream (MA02)**: `polynomial` and `reed-solomon` packages depend on
  this package for coefficient arithmetic.

## Spec

See [`code/specs/MA01-gf256.md`](../../../specs/MA01-gf256.md) for the full
specification, including the table construction algorithm, operation
definitions, and cross-language test vectors.
