# Changelog — polynomial-native

## [0.1.0] — 2026-04-03

### Added

- Initial release of the `coding-adventures-polynomial-native` Python extension.
- Wraps the Rust `polynomial` crate via `python-bridge` with zero third-party
  dependencies (no PyO3, no bindgen).
- Exposes a module-level API of 12 free functions matching the Rust `polynomial`
  crate's public API:
  - `normalize(poly)` — strip trailing near-zero coefficients
  - `degree(poly)` — degree of a polynomial
  - `zero()` — the zero polynomial `[0.0]`
  - `one()` — the multiplicative identity `[1.0]`
  - `add(a, b)` — polynomial addition
  - `subtract(a, b)` — polynomial subtraction
  - `multiply(a, b)` — polynomial multiplication via convolution
  - `divmod_poly(dividend, divisor)` — long division → (quotient, remainder)
  - `divide(a, b)` — quotient only
  - `modulo(a, b)` — remainder only
  - `evaluate(poly, x)` — Horner's method evaluation
  - `gcd(a, b)` — Euclidean GCD algorithm
- Rust panics (division by zero polynomial) are caught via
  `std::panic::catch_unwind` and raised as Python `ValueError`.
- Python `list[float]` ↔ Rust `&[f64]` marshalling handles both `float` and
  `int` elements for ergonomic use.
- `divmod_poly` returns a Python 2-tuple `(quotient, remainder)`.
- `BUILD` file for the build tool test runner.
- Comprehensive test suite with 30+ test cases covering all functions,
  error conditions, and mathematical properties.
