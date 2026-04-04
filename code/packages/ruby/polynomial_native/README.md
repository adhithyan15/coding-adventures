# coding_adventures_polynomial_native

A Ruby native extension wrapping the [`polynomial`](../../rust/polynomial/) Rust crate.
Provides polynomial arithmetic over `f64` coefficient arrays, exposed as module-level
functions under `CodingAdventures::PolynomialNative`.

## What is a Polynomial?

A polynomial is a mathematical expression like:

```
p(x) = 3 + 2x + x²
```

We store polynomials as Ruby `Array<Float>` where **the array index equals the degree**
of that term's coefficient (lowest degree first):

```ruby
[3.0, 2.0, 1.0]   # => 3 + 2x + x²
[1.0, 0.0, 5.0]   # => 1 + 5x²
[]                  # => the zero polynomial
```

This "little-endian" layout makes addition trivially position-aligned and enables
Horner's method for efficient evaluation.

## Where This Fits

Polynomial arithmetic is the foundation of three important algorithms in the
coding-adventures stack:

1. **GF(2^8) (MA01)** — Every GF(256) element is a polynomial over GF(2).
2. **Reed-Solomon error correction (MA02)** — Encoding/decoding uses polynomial
   multiplication and the extended Euclidean GCD.
3. **CRCs and checksums** — A CRC is the remainder of polynomial division over GF(2).

## Installation

```bash
gem install coding_adventures_polynomial_native
```

Requires Rust (via `cargo`) to be installed for native compilation.

## Usage

```ruby
require "coding_adventures_polynomial_native"

M = CodingAdventures::PolynomialNative

# Fundamentals
M.normalize([1.0, 0.0, 0.0])    # => [1.0]   (strip trailing zeros)
M.degree([3.0, 0.0, 2.0])       # => 2       (highest non-zero term index)
M.zero                           # => [0.0]
M.one                            # => [1.0]

# Arithmetic
a = [1.0, 2.0, 3.0]   # 1 + 2x + 3x²
b = [4.0, 5.0]         # 4 + 5x
M.add(a, b)            # => [5.0, 7.0, 3.0]   (5 + 7x + 3x²)
M.subtract(a, b)       # => [-3.0, -3.0, 3.0]
M.multiply(a, b)       # => [4.0, 13.0, 22.0, 15.0]

# Division
dividend = [5.0, 1.0, 3.0, 2.0]   # 5 + x + 3x² + 2x³
divisor  = [2.0, 1.0]              # 2 + x
q, r = M.divmod_poly(dividend, divisor)
# q => [3.0, -1.0, 2.0]  (3 - x + 2x²)
# r => [-1.0]             (remainder)

M.divide(dividend, divisor)   # => quotient only
M.modulo(dividend, divisor)   # => remainder only

# Evaluation using Horner's method — O(n), no exponentiation
# p(x) = 3 + 0x + x² evaluated at x = 2: 3 + 0 + 4 = 7
M.evaluate([3.0, 0.0, 1.0], 2.0)   # => 7.0

# Greatest common divisor (Euclidean algorithm)
a = [2.0, -3.0, 1.0]   # (x-1)(x-2)
b = [-1.0, 1.0]         # (x-1)
M.gcd(a, b)             # => proportional to [-1.0, 1.0]  (the factor x-1)
```

## Module as Mixin

`define_module_function` makes each function usable as both a module method and a
free function when the module is included:

```ruby
include CodingAdventures::PolynomialNative
normalize([1.0, 0.0])   # works without the M. prefix
```

## API Reference

| Function | Signature | Description |
|---|---|---|
| `normalize` | `(poly) -> Array` | Strip trailing near-zero coefficients |
| `degree` | `(poly) -> Integer` | Highest non-zero term index (0 for zero poly) |
| `zero` | `() -> Array` | The zero polynomial `[0.0]` |
| `one` | `() -> Array` | The unit polynomial `[1.0]` |
| `add` | `(a, b) -> Array` | Term-by-term addition |
| `subtract` | `(a, b) -> Array` | Term-by-term subtraction |
| `multiply` | `(a, b) -> Array` | Polynomial multiplication (convolution) |
| `divmod_poly` | `(a, b) -> [Array, Array]` | Long division: `[quotient, remainder]` |
| `divide` | `(a, b) -> Array` | Quotient of polynomial division |
| `modulo` | `(a, b) -> Array` | Remainder of polynomial division |
| `evaluate` | `(poly, x) -> Float` | Evaluate at x using Horner's method |
| `gcd` | `(a, b) -> Array` | Greatest common divisor (Euclidean) |

`divmod_poly`, `divide`, and `modulo` raise `ArgumentError` if the divisor is the zero
polynomial.

## Implementation Notes

- Built with `ruby-bridge` — zero dependencies beyond `libruby`, no Magnus or rb-sys.
- Panics from the Rust `polynomial::divmod` are caught with `std::panic::catch_unwind`
  and converted to Ruby `ArgumentError` exceptions.
- Float extraction uses `rb_num2dbl`, which handles Ruby Float, Integer, and Rational.
- The near-zero threshold for normalization is `f64::EPSILON × 10^6 ≈ 2.22 × 10^-10`.
