# GF256Native

A Swift package providing GF(2^8) finite field arithmetic (add, subtract,
multiply, divide, power, inverse) by calling into the Rust `gf256-c` static
library via **compile-time C linkage**.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What Is GF(256)?

**GF(2^8)** — "Galois Field of 256 elements" — is a finite field where the
elements are the bytes 0 through 255. The arithmetic is very different from
ordinary integer arithmetic:

| Operation | Rule |
|-----------|------|
| Add       | Bitwise XOR |
| Subtract  | Same as add (characteristic 2: −x = x) |
| Multiply  | Log + antilog table lookup |
| Divide    | Log subtraction + antilog lookup |
| Power     | Log scaling + antilog lookup |
| Inverse   | `ALOG[255 − LOG[a]]` |

Key property: **no element ever overflows**. `0xFF × 0xFF` produces a byte,
not a larger integer. The field is "closed" under all operations.

## Why This Package?

GF(256) is the mathematical foundation of three important real-world systems:

1. **Reed-Solomon error correction** — used in QR codes, CDs, DVDs, hard drives,
   and deep-space communication. Data bytes are GF(256) elements; encoding adds
   redundancy using polynomial arithmetic over GF(256).

2. **QR codes** — A QR code's error correction codewords are a Reed-Solomon
   code over GF(256). This lets a QR code survive up to 30% physical damage.

3. **AES encryption** — The SubBytes (S-box) step and MixColumns step use
   GF(256) arithmetic. (AES uses polynomial `0x11B`; this package uses `0x11D`.)

## The Primitive Polynomial

GF(256) elements are polynomials over GF(2) with degree ≤ 7:

```
a₇x⁷ + a₆x⁶ + … + a₁x + a₀,   each aᵢ ∈ {0, 1}
```

When multiplication produces degree ≥ 8, we reduce modulo an irreducible
polynomial of degree 8. This package uses:

```
p(x) = x^8 + x^4 + x^3 + x^2 + 1   =   0x11D   =   285
```

This polynomial is both irreducible (cannot be factored) and primitive (the
element `g = 2` generates all 255 non-zero elements).

## How the C Interop Works

Swift uses **compile-time C linkage** (not runtime FFI):

```
Rust source code (gf256-c crate)
    ↓  cargo build --release
libgf256_c.a  (static library, C ABI)
    ↓  copied to Sources/CGF256/
Swift Package Manager links .a into binary
    ↓  at compile time
Swift binary calls C functions directly
```

The GF(256) functions are all scalar operations (`uint8_t` inputs and outputs),
which map cleanly to Swift's `UInt8` type with no buffer management needed.

## Prerequisites

The Swift package requires the Rust static library to be compiled first:

```bash
# Step 1: Compile the Rust static library
cd code/packages/rust/gf256-c
cargo build --release

# Step 2: Copy into the Swift C target directory
cp target/release/libgf256_c.a \
   ../../swift/gf256-native/Sources/CGF256/

# Step 3: Build and test
cd ../../swift/gf256-native
swift test
```

Without the `.a` file, you will see: `library not found for -lgf256_c`.

## Usage

```swift
import GF256Native

// Addition (bitwise XOR)
GF256Native.add(0x53, 0xCA)        // → 0x99
GF256Native.add(7, 7)              // → 0  (every element is its own inverse)

// Subtraction (same as add in GF(256))
GF256Native.subtract(0x99, 0xCA)   // → 0x53

// Multiplication (log/antilog tables)
GF256Native.multiply(2, 64)        // → 128  (no reduction needed)
GF256Native.multiply(2, 128)       // → 29   (reduced mod 0x11D)
GF256Native.multiply(0, 255)       // → 0    (zero annihilates)

// Division (returns nil if divisor is zero)
GF256Native.divide(10, 2)          // → Optional(5)
GF256Native.divide(42, 0)          // → nil  (division by zero)

// Power
GF256Native.power(2, 8)            // → 29   (2^8 mod 0x11D)
GF256Native.power(2, 255)          // → 1    (generator cycles back to 1)
GF256Native.power(0, 0)            // → 1    (convention: 0^0 = 1)

// Inverse (returns nil for zero, which has no inverse)
GF256Native.inverse(1)             // → Optional(1)   (1 is its own inverse)
GF256Native.inverse(2)             // → Optional(142) (2 × 142 = 1 in GF256)
GF256Native.inverse(0)             // → nil

// Verify: a × inverse(a) = 1 for all non-zero a
if let inv = GF256Native.inverse(53) {
    GF256Native.multiply(53, inv)  // → 1
}

// Constants
GF256Native.zero                   // → 0 (additive identity)
GF256Native.one                    // → 1 (multiplicative identity)
GF256Native.primitivePolynomial    // → 285 = 0x11D
```

## API Reference

| Function | Description |
|----------|-------------|
| `GF256Native.add(_:_:)` | Add two elements (XOR), no error cases |
| `GF256Native.subtract(_:_:)` | Subtract (same as add), no error cases |
| `GF256Native.multiply(_:_:)` | Multiply via log tables, no error cases |
| `GF256Native.divide(_:_:)` | Divide; returns `nil` if divisor is zero |
| `GF256Native.power(_:_:)` | Raise to non-negative integer power |
| `GF256Native.inverse(_:)` | Multiplicative inverse; returns `nil` for zero |

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `GF256Native.zero` | `0` | Additive identity |
| `GF256Native.one` | `1` | Multiplicative identity |
| `GF256Native.primitivePolynomial` | `285` | Field-defining polynomial |

## Error Handling

Operations that are mathematically undefined return `nil`:

- `divide(a, 0)` — division by zero
- `inverse(0)` — zero has no multiplicative inverse

All other operations are defined for every pair of `UInt8` inputs and always
return a `UInt8` result in `[0, 255]`.

## How It Fits in the Stack

```
MA00: Polynomial — coefficient-array polynomial arithmetic over f64
  ↓
MA01: GF256 (this package) — Galois Field GF(2^8) arithmetic
  ↓
MA02: Reed-Solomon — error-correcting codes over GF(256)
```

## Design Decisions

1. **Enum namespace**: `public enum GF256Native` (no cases) prevents
   instantiation and clearly scopes all functions.

2. **Optional for error cases**: `divide(_:_:)` and `inverse(_:)` return
   `UInt8?` rather than throwing. This forces the caller to handle the
   error at the call site, which is idiomatic Swift.

3. **Per-thread error flag in Rust**: The Rust C layer uses a `thread_local!`
   boolean to signal errors (instead of return codes, which would conflict with
   the sentinel `0xFF` return value). Swift checks this flag immediately after
   each potentially-failing call.

4. **No buffer management**: Unlike polynomial operations, all GF(256) functions
   take and return scalar `UInt8` values. There is no buffer protocol, no
   `withUnsafeBufferPointer`, and no output capacity to manage.

## Development

```bash
# First, build and install the Rust library (from repo root):
cd code/packages/rust/gf256-c
cargo build --release
cp target/release/libgf256_c.a \
   ../../swift/gf256-native/Sources/CGF256/

# Then run Swift tests:
cd ../../swift/gf256-native
swift test

# Build only:
swift build

# Verbose test output:
swift test --verbose
```

## License

Part of the coding-adventures project.
