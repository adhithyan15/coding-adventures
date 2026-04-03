# Changelog — @coding-adventures/polynomial

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of polynomial arithmetic over real numbers.
- `normalize(p)` — strip trailing zero coefficients.
- `degree(p)` — highest non-zero index; -1 for zero polynomial.
- `zero()` / `one()` — additive and multiplicative identity polynomials.
- `add(a, b)` / `subtract(a, b)` — term-by-term arithmetic.
- `multiply(a, b)` — polynomial convolution.
- `divmod(a, b)` — polynomial long division, returns `[quotient, remainder]`.
- `divide(a, b)` / `mod(a, b)` — convenience wrappers for divmod.
- `evaluate(p, x)` — Horner's method for fast evaluation.
- `gcd(a, b)` — Euclidean GCD algorithm.
- Comprehensive test suite with >95% coverage.
- Literate programming comments explaining each algorithm with worked examples.
