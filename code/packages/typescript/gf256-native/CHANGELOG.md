# Changelog — @coding-adventures/gf256-native

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-03

### Added

- Initial release: native Node.js addon wrapping the Rust `gf256` crate via `node-bridge` N-API FFI.
- Exposes all six GF(2^8) field operations as free functions:
  - `add(a, b)` — bitwise XOR (characteristic-2 addition)
  - `subtract(a, b)` — identical to add (XOR) in characteristic-2
  - `multiply(a, b)` — log/antilog table multiplication with modular reduction
  - `divide(a, b)` — throws if b == 0
  - `power(base, exp)` — exponentiation via log tables, handles 0^0=1 convention
  - `inverse(a)` — multiplicative inverse, throws if a == 0
- Exports three module-level constants:
  - `ZERO` = 0 (additive identity)
  - `ONE` = 1 (multiplicative identity)
  - `PRIMITIVE_POLYNOMIAL` = 285 = 0x11D (irreducible polynomial x^8+x^4+x^3+x^2+1)
- `std::panic::catch_unwind` used to turn Rust panics (divide/inverse of zero) into JS exceptions.
- Full TypeScript declarations in `index.d.ts`.
- 35+ unit tests in `tests/gf256_native.test.ts` using Vitest, covering all operations and edge cases.
- `BUILD` file for integration with the repo's Go-based build tool.
