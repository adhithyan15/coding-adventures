# coding_adventures_gf256_native

A Ruby native extension wrapping the [`gf256`](../../rust/gf256/) Rust crate.
Provides arithmetic in **GF(2^8)** — the Galois Field with 256 elements — as module-level
functions under `CodingAdventures::GF256Native`.

## What is GF(2^8)?

GF(2^8) — "Galois Field of 256 elements" — is a finite field where:

- Elements are the integers 0 through 255 (bytes).
- **Addition = XOR** (characteristic 2: every element is its own additive inverse).
- **Multiplication** is polynomial multiplication modulo the irreducible polynomial
  `p(x) = x^8 + x^4 + x^3 + x^2 + 1` (= 0x11D = 285).

```
add(0x53, 0xCA)      = 0x53 XOR 0xCA  = 0x99
subtract(a, b)        = add(a, b)       (same operation!)
multiply(2, 128)      = 29             (128 * 2 overflows; XOR with 0x11D)
inverse(2)            = 142            (2 * 142 = 1 in GF(256))
```

## Where This Fits

GF(2^8) is the arithmetic foundation for:

1. **Reed-Solomon error correction** — QR codes, CDs, DVDs, hard drives, deep-space
   communication. Every data byte is a GF(256) element; RS codes add redundancy by
   computing polynomial syndromes over GF(256).
2. **AES encryption** — The SubBytes (S-box) and MixColumns steps use GF(2^8)
   arithmetic. (AES uses 0x11B instead of 0x11D for its polynomial.)

## Installation

```bash
gem install coding_adventures_gf256_native
```

Requires Rust (via `cargo`) to be installed for native compilation.

## Usage

```ruby
require "coding_adventures_gf256_native"

M = CodingAdventures::GF256Native

# Constants
M::ZERO                # => 0
M::ONE                 # => 1
M::PRIMITIVE_POLYNOMIAL  # => 285 (= 0x11D)

# Addition = XOR
M.add(0x53, 0xCA)      # => 0x99
M.add(42, 42)          # => 0   (every element is its own inverse)

# Subtraction = XOR (same as addition in characteristic 2)
M.subtract(0x53, 0xCA) # => 0x99  (same as add!)

# Multiplication via log/antilog tables — O(1)
M.multiply(2, 4)       # => 8
M.multiply(2, 128)     # => 29    (overflow, reduced mod 0x11D)
M.multiply(0, 255)     # => 0     (zero times anything is zero)

# Division
M.divide(8, 4)         # => 2
M.divide(0, 5)         # => 0     (zero divided by anything is zero)
M.divide(1, 0)         # raises ArgumentError (division by zero)

# Exponentiation
M.power(2, 8)          # => 29    (2^8 in GF(256))
M.power(2, 255)        # => 1     (the multiplicative group has order 255)
M.power(0, 0)          # => 1     (by convention)

# Multiplicative inverse: a * inverse(a) = 1
M.inverse(1)           # => 1     (1 is its own inverse)
M.inverse(2)           # => 142   (2 * 142 = 1 in GF(256))
M.inverse(0)           # raises ArgumentError (zero has no inverse)
```

## API Reference

| Function | Signature | Description |
|---|---|---|
| `add` | `(a, b) -> Integer` | GF(256) addition: a XOR b |
| `subtract` | `(a, b) -> Integer` | GF(256) subtraction: a XOR b (= add) |
| `multiply` | `(a, b) -> Integer` | GF(256) multiplication via log tables |
| `divide` | `(a, b) -> Integer` | GF(256) division; raises `ArgumentError` if b == 0 |
| `power` | `(base, exp) -> Integer` | GF(256) exponentiation; exp must be >= 0 |
| `inverse` | `(a) -> Integer` | Multiplicative inverse; raises `ArgumentError` if a == 0 |

### Constants

| Constant | Value | Description |
|---|---|---|
| `ZERO` | `0` | Additive identity |
| `ONE` | `1` | Multiplicative identity |
| `PRIMITIVE_POLYNOMIAL` | `285` (0x11D) | Irreducible polynomial used for reduction |

All arguments must be Ruby Integers in the range 0..=255. Out-of-range values raise
`ArgumentError` with an informative message.

## Implementation Notes

- Built with `ruby-bridge` — zero dependencies beyond `libruby`, no Magnus or rb-sys.
- Multiplication is O(1) using log/antilog tables initialized lazily via
  `std::sync::OnceLock` (thread-safe, initialized at most once).
- Panics from `gf256::divide` and `gf256::inverse` are caught with
  `std::panic::catch_unwind` and converted to Ruby `ArgumentError` exceptions.
- Module constants are set via `rb_define_const` called from `Init_gf256_native`.
