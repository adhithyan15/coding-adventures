# Changelog — @coding-adventures/polynomial-native

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-03

### Added

- Initial release: native Node.js addon wrapping the Rust `polynomial` crate via `node-bridge` N-API FFI.
- Exposes all twelve polynomial operations as free functions (not a class):
  - `normalize(poly)` — strip trailing near-zero coefficients
  - `degree(poly)` — index of the highest non-zero coefficient
  - `zero()` — the zero polynomial `[0.0]`
  - `one()` — the one polynomial `[1.0]`
  - `add(a, b)` — term-by-term addition
  - `subtract(a, b)` — term-by-term subtraction
  - `multiply(a, b)` — polynomial convolution
  - `divmodPoly(dividend, divisor)` — long division returning `[quotient, remainder]`
  - `divide(a, b)` — quotient only
  - `modulo(a, b)` — remainder only
  - `evaluate(poly, x)` — Horner's method evaluation at a point
  - `gcd(a, b)` — Euclidean GCD of two polynomials
- `std::panic::catch_unwind` used to turn Rust panics (e.g. divide-by-zero) into JS exceptions.
- Full TypeScript declarations in `index.d.ts`.
- 35+ unit tests in `tests/polynomial_native.test.ts` using Vitest.
- `BUILD` file for integration with the repo's Go-based build tool.
