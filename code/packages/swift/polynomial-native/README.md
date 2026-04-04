# PolynomialNative

A Swift package providing polynomial arithmetic (add, subtract, multiply, divide,
evaluate, GCD) by calling into the Rust `polynomial-c` static library via
**compile-time C linkage**.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What Is Polynomial Arithmetic?

A **polynomial** is a mathematical expression with a variable `x`:

```
p(x) = a₀ + a₁x + a₂x² + … + aₙxⁿ
```

We represent polynomials as `[Double]` arrays where the **array index equals
the degree** of that term's coefficient:

```swift
[3.0, 0.0, 2.0]   // 3 + 0·x + 2·x²  =  3 + 2x²
[1.0, 2.0, 3.0]   // 1 + 2x + 3x²
[]                 // the zero polynomial
```

This "lowest degree first" (little-endian) layout makes addition trivial:
`result[i] = a[i] + b[i]`.

## Why This Package?

Polynomial arithmetic is the mathematical foundation of three important
algorithms in the coding-adventures stack:

- **GF(256) (MA01)** — The Galois Field used by Reed-Solomon and AES is defined
  as a polynomial ring modulo an irreducible polynomial.
- **Reed-Solomon error correction (MA02)** — Encoding is polynomial multiplication;
  decoding uses the extended Euclidean GCD algorithm.
- **CRCs and checksums** — A CRC is the remainder of polynomial division over GF(2).

## How the C Interop Works

Swift doesn't have a "native extension" mechanism like Python (`ctypes`) or
Ruby (`fiddle`). Instead, Swift uses **compile-time C linkage**:

```
Rust source code
    ↓  cargo build --release
libpolynomial_c.a  (static library, C ABI)
    ↓  copied to Sources/CPolynomial/
Swift Package Manager links .a into binary
    ↓  at compile time
Swift binary calls C functions directly
```

The key files:

| File | Role |
|------|------|
| `polynomial_c.h` | Declares C function signatures |
| `module.modulemap` | Tells SPM "CPolynomial module = this header" |
| `libpolynomial_c.a` | Compiled Rust code (not in repo, build first) |
| `PolynomialNative.swift` | Swift wrapper converting `[Double]` ↔ C buffers |

## Prerequisites

The Swift package requires the Rust static library to be compiled first:

```bash
# Step 1: Compile the Rust static library
cd code/packages/rust/polynomial-c
cargo build --release

# Step 2: Copy into the Swift C target directory
cp target/release/libpolynomial_c.a \
   ../../swift/polynomial-native/Sources/CPolynomial/

# Step 3: Build and test
cd ../../swift/polynomial-native
swift test
```

Without the `.a` file, you will see: `library not found for -lpolynomial_c`.

## Usage

```swift
import PolynomialNative

// Create polynomials as [Double] arrays (index = degree)
let p = [1.0, 2.0, 3.0]   // 1 + 2x + 3x²
let q = [4.0, 5.0]         // 4 + 5x

// Normalize (strip trailing near-zero coefficients)
Polynomial.normalize([1.0, 0.0, 0.0])  // → [1.0]

// Degree
Polynomial.degree([3.0, 0.0, 2.0])  // → 2

// Evaluate at x using Horner's method
Polynomial.evaluate([3.0, 0.0, 2.0], at: 2.0)  // → 11.0  (3 + 2·4 = 11)

// Addition
Polynomial.add(p, q)        // → [5.0, 7.0, 3.0]  (5 + 7x + 3x²)

// Subtraction
Polynomial.subtract(p, q)   // → [-3.0, -3.0, 3.0]

// Multiplication
Polynomial.multiply([1.0, 2.0], [3.0, 4.0])
// → [3.0, 10.0, 8.0]  =  (1 + 2x)(3 + 4x) = 3 + 10x + 8x²

// Division (returns nil if divisor is zero polynomial)
if let (quotient, remainder) = Polynomial.divmod([5, 1, 3, 2], [2, 1]) {
    // quotient  = [3, -1, 2]  =  3 − x + 2x²
    // remainder = [-1]
}

// Just quotient or remainder
let q = Polynomial.divide([5, 1, 3, 2], [2, 1])   // → Optional([3, -1, 2])
let r = Polynomial.modulo([5, 1, 3, 2], [2, 1])   // → Optional([-1])

// GCD
Polynomial.gcd([2.0, -3.0, 1.0], [-1.0, 1.0])
// → [-1.0, 1.0]  =  x − 1  (divides both inputs)
```

## API Reference

| Function | Description |
|----------|-------------|
| `Polynomial.normalize(_:)` | Strip trailing near-zero coefficients |
| `Polynomial.degree(_:)` | Index of highest non-zero coefficient |
| `Polynomial.evaluate(_:at:)` | Evaluate at x using Horner's method |
| `Polynomial.add(_:_:)` | Term-by-term addition |
| `Polynomial.subtract(_:_:)` | Term-by-term subtraction |
| `Polynomial.multiply(_:_:)` | Polynomial convolution |
| `Polynomial.divmod(_:_:)` | Long division → `(quotient, remainder)?` |
| `Polynomial.divide(_:_:)` | Long division → quotient only (`[Double]?`) |
| `Polynomial.modulo(_:_:)` | Long division → remainder only (`[Double]?`) |
| `Polynomial.gcd(_:_:)` | Euclidean GCD algorithm |

All functions returning `[Double]?` return `nil` if the divisor is the zero
polynomial (mathematically undefined operation).

## How It Fits in the Stack

```
MA00: Polynomial (this package)
  ↓
MA01: GF256 — uses polynomial ring modulo primitive polynomial
  ↓
MA02: Reed-Solomon — polynomial operations over GF(256)
```

## Design Decisions

1. **Enum namespace**: `public enum Polynomial` (no cases) prevents instantiation
   and provides clear scoping for all functions.

2. **Returning `Optional` for division errors**: Rather than throwing or crashing,
   division by zero returns `nil`. This is idiomatic Swift and forces callers to
   handle the error case at the call site.

3. **Buffer protocol for C interop**: The Rust C layer uses caller-provided output
   buffers (no cross-FFI heap allocation). Swift's `withUnsafeBufferPointer` borrows
   the array's internal storage without copying, then we truncate to actual length.

4. **Two-step build**: The `.a` file is not committed to the repo. Users build it
   from the Rust source. This keeps binary artifacts out of git while maintaining
   a fully reproducible build.

## Development

```bash
# First, build and install the Rust library (from repo root):
cd code/packages/rust/polynomial-c
cargo build --release
cp target/release/libpolynomial_c.a \
   ../../swift/polynomial-native/Sources/CPolynomial/

# Then run Swift tests:
cd ../../swift/polynomial-native
swift test

# Build only:
swift build

# Verbose test output:
swift test --verbose
```

## License

Part of the coding-adventures project.
