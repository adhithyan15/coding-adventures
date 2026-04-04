# CodingAdventures::GF256

Arithmetic in the Galois Field GF(2⁸) — 256-element finite field arithmetic
implemented in Pure Perl, part of the coding-adventures monorepo.

## What Is GF(2⁸)?

GF(2⁸) is the unique finite field with 256 elements. Every element fits in a
single byte (0–255). It is the foundation of:

- **AES** — the S-box and MixColumns steps use GF(2⁸) arithmetic
- **Reed-Solomon codes** — used in QR codes, CDs, DVDs, RAID-6
- **CRC polynomials** — many CRC standards use polynomial arithmetic over GF(2)

## How It Works

Elements are polynomials over GF(2) with degree ≤ 7, represented as integers
0–255 where bit `i` is the coefficient of `x^i`. All arithmetic is done modulo
the primitive polynomial x⁸ + x⁴ + x³ + x + 1 (0x11D).

**Addition** is bitwise XOR — no carries because the field has characteristic 2.

**Multiplication** uses precomputed LOG and ALOG tables for O(1) operations:
```
a * b = ALOG[(LOG[a] + LOG[b]) mod 255]
```

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::GF256 qw(add subtract multiply divide power inverse);

# Addition is XOR
my $sum  = add(0x53, 0xCA);        # = 0x99

# AES known-good: 0x53 * 0xCA = 1 (they are multiplicative inverses)
my $prod = multiply(0x53, 0xCA);   # = 1

# Division
my $quot = divide(0x53, 0x02);

# Power: generator order is 255
my $one  = power(2, 255);          # = 1

# Inverse: a * inverse(a) = 1
my $inv  = inverse(0x53);          # = 0xCA
```

## Functions

| Function | Description |
|---|---|
| `add($a, $b)` | Bitwise XOR |
| `subtract($a, $b)` | Same as add (characteristic 2) |
| `multiply($a, $b)` | Field multiplication via LOG/ALOG |
| `divide($a, $b)` | Field division; dies if $b == 0 |
| `power($base, $exp)` | Exponentiation; $exp must be non-negative |
| `inverse($a)` | Multiplicative inverse; dies if $a == 0 |

## How It Fits in the Stack

`GF256` is a foundational mathematics package (layer MA01). It builds on
the polynomial arithmetic concepts from `Polynomial` (MA00), but restricts
to GF(2) coefficients and uses integer representations optimized for bytes.

## Running Tests

```bash
prove -l -v t/
```

## License

MIT
