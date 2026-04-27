# Changelog — gf256-native

## [0.1.0] — 2026-04-03

### Added

- Initial release of the `coding-adventures-gf256-native` Python extension.
- Wraps the Rust `gf256` crate via `python-bridge` with zero third-party
  dependencies (no PyO3, no bindgen).
- Exposes a module-level API of 6 free functions plus 3 constants:
  - `add(a, b)` — XOR addition (characteristic 2)
  - `subtract(a, b)` — XOR subtraction (same as add)
  - `multiply(a, b)` — log/antilog table multiplication
  - `divide(a, b)` — field division, raises `ValueError` if `b == 0`
  - `power(base, exp)` — GF(256) exponentiation
  - `inverse(a)` — multiplicative inverse, raises `ValueError` if `a == 0`
  - `ZERO = 0` — additive identity
  - `ONE = 1` — multiplicative identity
  - `PRIMITIVE_POLYNOMIAL = 0x11D` — the irreducible reduction polynomial
- All arguments validated to be integers in range [0, 255].
- Rust panics (division by zero, inverse of zero) caught via
  `std::panic::catch_unwind` and raised as Python `ValueError`.
- `BUILD` file for the build tool test runner.
- Comprehensive test suite with 35+ test cases covering all functions,
  constants, mathematical properties, and error conditions.
