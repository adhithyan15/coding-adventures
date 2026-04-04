# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-03

### Added

- `GF256Native.add(_:_:)` — Adds two GF(256) elements (bitwise XOR).
- `GF256Native.subtract(_:_:)` — Subtracts two GF(256) elements (same as add
  in characteristic-2 fields).
- `GF256Native.multiply(_:_:)` — Multiplies two GF(256) elements using
  log/antilog table lookup (O(1)).
- `GF256Native.divide(_:_:)` — Divides two GF(256) elements; returns `nil` if
  divisor is zero.
- `GF256Native.power(_:_:)` — Raises a GF(256) element to a non-negative integer
  power using log-table scaling.
- `GF256Native.inverse(_:)` — Computes the multiplicative inverse; returns `nil`
  if input is zero.
- `GF256Native.zero` — The additive identity (0).
- `GF256Native.one` — The multiplicative identity (1).
- `GF256Native.primitivePolynomial` — The field's defining polynomial (285 = 0x11D).
- `CGF256` system library target — wraps the C header and module map for
  compile-time C interop via Swift Package Manager.
- Comprehensive test suite covering all field operations, algebraic laws
  (commutativity, associativity, distributivity), field axioms, and the
  generator property of element 2.
- Literate programming style explaining GF(256) arithmetic, the log/antilog
  approach, the characteristic-2 field, and the primitive polynomial.
- `required_capabilities.json` — documents no special OS capabilities needed.

### Architecture

This package uses compile-time C linkage (not runtime FFI/dynamic loading):

1. Rust crate `gf256-c` exports C ABI functions via `#[no_mangle] pub extern "C"`.
2. The compiled static library (`libgf256_c.a`) is linked into the Swift binary.
3. A C module map (`module.modulemap`) lets Swift `import CGF256` directly.
4. Error handling: Rust catches panics from undefined ops (divide/inverse by zero)
   and returns a sentinel + per-thread error flag. Swift checks the flag and
   converts to `Optional` (returning `nil` on error).
