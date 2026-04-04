# Changelog — coding-adventures-polynomial (Lua)

All notable changes to the Lua `polynomial` package are documented here.

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of `coding_adventures.polynomial` (MA00).
- `normalize(poly)` — strips trailing near-zero coefficients (threshold 1e-10); always returns at least `{0}`.
- `degree(poly)` — returns the index of the highest non-zero coefficient minus one; returns -1 for the zero polynomial.
- `zero()` — returns the additive identity `{0}`.
- `one()` — returns the multiplicative identity `{1}`.
- `add(a, b)` — term-by-term addition with implicit zero-padding of the shorter operand.
- `subtract(a, b)` — term-by-term subtraction; result is normalized.
- `multiply(a, b)` — polynomial convolution; degree of result = deg(a) + deg(b).
- `divmod(dividend, divisor)` — polynomial long division returning `(quotient, remainder)`; errors on division by the zero polynomial.
- `divide(a, b)` — quotient-only wrapper around `divmod`.
- `modulo(a, b)` — remainder-only wrapper around `divmod`.
- `evaluate(poly, x)` — evaluates a polynomial at `x` using Horner's method (O(n) without exponentiation).
- `gcd(a, b)` — Euclidean GCD algorithm; returns the normalized GCD polynomial.
- Knuth-style literate comments throughout the source with diagrams, step-by-step worked examples, and explanations suitable for beginners.
- 49 busted unit tests covering all operations, edge cases, and the module API.
