# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-03

### Added

- `Polynomial.normalize(_:)` — Strips trailing near-zero coefficients from a
  polynomial array via the Rust `polynomial-c` C ABI.
- `Polynomial.degree(_:)` — Returns the degree (index of highest non-zero
  coefficient) of a polynomial.
- `Polynomial.evaluate(_:at:)` — Evaluates a polynomial at x using Horner's
  method (O(n), no exponentiation).
- `Polynomial.add(_:_:)` — Term-by-term polynomial addition.
- `Polynomial.subtract(_:_:)` — Term-by-term polynomial subtraction.
- `Polynomial.multiply(_:_:)` — Polynomial multiplication by convolution.
- `Polynomial.divide(_:_:)` — Returns the quotient of polynomial long division,
  or `nil` if the divisor is the zero polynomial.
- `Polynomial.modulo(_:_:)` — Returns the remainder of polynomial long division,
  or `nil` if the divisor is the zero polynomial.
- `Polynomial.divmod(_:_:)` — Returns `(quotient, remainder)` as a named tuple,
  or `nil` on zero divisor. Provides reliable error detection.
- `Polynomial.gcd(_:_:)` — Greatest common divisor via the Euclidean algorithm.
- `CPolynomial` system library target — wraps the C header and module map for
  compile-time C interop via Swift Package Manager.
- Comprehensive test suite covering normalization, degree, evaluation,
  addition, subtraction, multiplication, division, remainder, GCD, and
  algebraic round-trip properties.
- Literate programming style with extensive inline documentation explaining
  the buffer protocol, pointer-borrowing idiom, and mathematical properties.
- `required_capabilities.json` — documents no special OS capabilities needed.

### Architecture

This package uses compile-time C linkage (not runtime FFI/dynamic loading):

1. Rust crate `polynomial-c` exports C ABI functions via `#[no_mangle] pub extern "C"`.
2. The compiled static library (`libpolynomial_c.a`) is linked into the Swift binary.
3. A C module map (`module.modulemap`) lets Swift `import CPolynomial` directly.
4. Swift wrapper code (`PolynomialNative.swift`) uses `withUnsafeBufferPointer`
   to borrow array storage without copying, passing pointer + length pairs to C.
