# Changelog — coding_adventures_polynomial_native

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-03

### Added

- Initial release: Ruby native extension wrapping the `polynomial` Rust crate.
- Module `CodingAdventures::PolynomialNative` with twelve module functions:
  - `normalize(poly)` — strip trailing near-zero coefficients
  - `degree(poly)` — return the degree (index of highest non-zero term)
  - `zero` — return the zero polynomial `[0.0]`
  - `one` — return the unit polynomial `[1.0]`
  - `add(a, b)` — term-by-term polynomial addition
  - `subtract(a, b)` — term-by-term polynomial subtraction
  - `multiply(a, b)` — polynomial multiplication via convolution
  - `divmod_poly(a, b)` — polynomial long division, returns `[quotient, remainder]`
  - `divide(a, b)` — returns the quotient of polynomial division
  - `modulo(a, b)` — returns the remainder of polynomial division
  - `evaluate(poly, x)` — evaluate using Horner's method (O(n), no exponentiation)
  - `gcd(a, b)` — greatest common divisor via the Euclidean algorithm
- Polynomial representation: Ruby `Array<Float>` where index equals degree.
- `divmod_poly`, `divide`, and `modulo` raise `ArgumentError` (not process abort)
  when the divisor is the zero polynomial, via `std::panic::catch_unwind`.
- Built via `cargo build --release` with zero dependencies beyond libruby.
- 40+ test cases covering fundamentals, arithmetic, division, evaluation, GCD,
  algebraic identities, and error conditions.
