# gf256

GF(2^8) — Galois Field arithmetic for Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer MA01.

## What It Does

This library implements arithmetic in GF(256), the finite field with 256 elements.
Elements are single bytes (`UInt8`, values 0..255). All operations stay within
this range — no overflow is possible.

GF(256) is used in:
- **Reed-Solomon error correction** — the math behind QR codes, CDs, and DVDs
- **AES encryption** — the SubBytes and MixColumns steps
- **Shamir's secret sharing** — splitting a secret among multiple parties

## Key Insight: Addition is XOR

In GF(2^8), addition is bitwise XOR. Each bit represents a polynomial
coefficient over GF(2), and GF(2) addition is `1 + 1 = 0` — which is XOR.

Consequence: every element is its own inverse (`a + a = 0`), so subtraction
equals addition.

## Primitive Polynomial

Multiplication is polynomial multiplication modulo the irreducible polynomial:

```
p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
```

## API

All functions live in the `GF256` enum namespace:

```swift
import GF256

// Constants
GF256.zero           // 0
GF256.one            // 1
GF256.primitivePoly  // 0x11D

// Arithmetic
GF256.add(0x53, 0xCA)       // 0x53 ^ 0xCA
GF256.subtract(0x53, 0xCA)  // same as add (XOR)
GF256.multiply(2, 128)      // 29 (= 0x1D, first reduction step)
GF256.divide(29, 2)         // 128

// Power and inverse
GF256.power(2, 8)    // 29  (2^8 mod p(x))
GF256.inverse(2)     // 142 (2^-1; verify: multiply(2, 142) == 1)

// Tables (read-only, built at startup)
GF256.ALOG[0]   // 1   (2^0)
GF256.ALOG[1]   // 2   (2^1)
GF256.ALOG[8]   // 29  (2^8 mod p(x))
GF256.LOG[2]    // 1   (log_2(2) = 1)
```

## Where It Fits

```
MA00 polynomial  (polynomial arithmetic over Double)
      ↓
MA01 gf256       ← you are here
      ↓
Reed-Solomon, QR codes, AES, ...
```

## Running Tests

```bash
swift test
```

## License

MIT
