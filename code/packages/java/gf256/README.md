# gf256 — Java

GF(2^8) Galois Field arithmetic for Java.

## What This Is

This package implements arithmetic in **GF(256)**, the finite field with exactly 256
elements. It is the foundation for Reed-Solomon error correction (QR codes, CDs, hard
drives) and AES encryption.

In GF(256):
- **Addition is XOR** — because the field has characteristic 2 (1 + 1 = 0)
- **Subtraction equals addition** — every element is its own additive inverse
- **Multiplication** uses precomputed log/antilog tables for O(1) performance
- **Every non-zero element has a multiplicative inverse**

## Usage

```java
import com.codingadventures.gf256.GF256;

// Add (XOR)
int sum = GF256.add(0x53, 0xCA);      // 0x99

// Multiply using log/antilog tables
int product = GF256.mul(0x53, 0x8C);  // 1 (they are multiplicative inverses)

// Divide
int quotient = GF256.div(4, 2);       // 2

// Power
int p = GF256.pow(2, 8);              // 29 (= 0x1D, after reduction by 0x11D)

// Inverse
int inv = GF256.inv(0x53);            // 0x8C
```

## Primitive Polynomial

All arithmetic uses the Reed-Solomon primitive polynomial:

```
p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
```

## Spec

See `code/specs/MA01-gf256.md` for the full specification including theory,
test vectors, and algorithm description.

## Tests

```
gradle test
```

Part of the [MA series](../../../../specs/MA01-gf256.md) — the math foundation
for 2D barcodes.
