# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-03

### Added

- `poly_c_normalize` — Normalize a polynomial (strip trailing near-zero
  coefficients) via a caller-provided output buffer.
- `poly_c_degree` — Return the degree of a polynomial (index of highest
  non-zero coefficient).
- `poly_c_evaluate` — Evaluate a polynomial at x using Horner's method.
- `poly_c_add` — Term-by-term polynomial addition.
- `poly_c_subtract` — Term-by-term polynomial subtraction.
- `poly_c_multiply` — Polynomial multiplication by convolution.
- `poly_c_divide` — Polynomial quotient; returns 0 elements on zero divisor.
- `poly_c_modulo` — Polynomial remainder; returns 0 elements on zero divisor.
- `poly_c_divmod` — Combined quotient and remainder; returns -1 on zero
  divisor, 0 on success. Uses separate caller-provided buffers for each result.
- `poly_c_gcd` — Greatest common divisor via the Euclidean algorithm.
- `include/polynomial_c.h` — C header declaring all exported functions with
  full documentation of the memory protocol, error handling, and buffer sizing.
- `Cargo.toml` with `crate-type = ["staticlib", "cdylib"]` and LTO enabled.
- `BUILD` file for the coding-adventures build tool.
- Literate programming style with extensive inline comments explaining the
  buffer protocol, `catch_unwind` usage, and safety contracts.
