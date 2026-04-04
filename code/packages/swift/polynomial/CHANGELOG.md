# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-03

### Added

- `Polynomial.normalize(_:)` — Strips trailing near-zero coefficients (threshold
  1e-10). Never returns empty: the zero polynomial is represented as `[0.0]`.
- `Polynomial.degree(_:)` — Returns the index of the highest non-zero coefficient.
  Zero polynomial returns degree 0.
- `Polynomial.zero()` — Returns the additive identity `[0.0]`.
- `Polynomial.one()` — Returns the multiplicative identity `[1.0]`.
- `Polynomial.add(_:_:)` — Term-by-term addition, normalized.
- `Polynomial.subtract(_:_:)` — Term-by-term subtraction, normalized.
- `Polynomial.multiply(_:_:)` — Polynomial convolution, normalized.
- `Polynomial.divmod(_:_:)` — Polynomial long division returning
  `(quotient, remainder)`. Uses `precondition` to guard against zero divisor.
- `Polynomial.divide(_:_:)` — Returns the quotient of `divmod`.
- `Polynomial.mod(_:_:)` — Returns the remainder of `divmod`. Used for
  reducing high-degree polynomials modulo a primitive polynomial (GF arithmetic).
- `Polynomial.evaluate(_:_:)` — Evaluates at a point using Horner's method
  (O(n) additions and multiplications, no `pow` calls).
- `Polynomial.gcd(_:_:)` — Euclidean GCD; result is made monic (leading
  coefficient normalized to 1.0) for uniqueness.
- Comprehensive test suite with 40+ test cases covering all functions,
  edge cases, and mathematical properties (commutativity, identity, round-trips).
- Literate programming style with extensive inline documentation explaining
  the math, algorithms, and design decisions.

### Notes

- Diverges from TypeScript reference in one way: the zero polynomial is
  represented as `[0.0]` (never empty). This avoids index-out-of-bounds in
  Swift when accessing the constant term.
- All functions are members of `public enum Polynomial` (used as a namespace)
  to avoid name conflicts with Swift standard library operators.
