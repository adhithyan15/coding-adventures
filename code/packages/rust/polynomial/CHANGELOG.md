# Changelog — polynomial

All notable changes to this package are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## 0.1.0 — Initial release

### Added

- `normalize(poly)` — strips trailing near-zero coefficients using threshold `f64::EPSILON * 1e6`
- `degree(poly)` — returns the degree of a normalized polynomial (0 for the zero polynomial)
- `zero()` — returns the zero polynomial `[0.0]`
- `one()` — returns the multiplicative identity `[1.0]`
- `add(a, b)` — term-by-term addition of two polynomials
- `subtract(a, b)` — term-by-term subtraction
- `multiply(a, b)` — polynomial multiplication via convolution (O(n·m))
- `divmod(dividend, divisor)` — polynomial long division returning `(quotient, remainder)`; panics on zero divisor
- `divide(a, b)` — quotient-only wrapper over `divmod`
- `modulo(a, b)` — remainder-only wrapper over `divmod` (named `modulo` since `mod` is a Rust keyword)
- `evaluate(poly, x)` — fast Horner's-method evaluation at a point
- `gcd(a, b)` — Euclidean GCD algorithm for polynomials
- Knuth-style literate comments throughout `src/lib.rs`
- 55+ integration tests in `tests/polynomial_test.rs`
