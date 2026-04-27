# Changelog — coding_adventures_polynomial

## 0.1.0 — 2026-04-24

### Added

- Initial release: coefficient-array polynomial arithmetic implementing spec MA00.
- `normalize` — strips trailing zeros from a polynomial.
- `polynomialDegree` — returns highest non-zero index, or -1 for zero polynomial.
- `polynomialZero` / `polynomialOne` — additive and multiplicative identities.
- `polynomialAdd` / `polynomialSubtract` — term-by-term coefficient arithmetic.
- `polynomialMultiply` — polynomial convolution (O(m·n)).
- `polynomialDivmod` — polynomial long division returning `(quotient, remainder)`.
- `polynomialDivide` / `polynomialMod` — convenience wrappers for divmod.
- `polynomialEvaluate` — Horner's method for O(n) evaluation.
- `polynomialGcd` — Euclidean GCD algorithm for polynomials.
- 43 unit tests covering all operations, algebraic properties, edge cases,
  and the spec's worked example (divide `[5,1,3,2]` by `[2,1]`).
