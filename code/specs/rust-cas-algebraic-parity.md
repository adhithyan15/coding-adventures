# Rust CAS algebraic parity

## Goal

Port the Python `cas-algebraic` package to Rust so the Rust symbolic/CAS stack
can factor integer univariate polynomials over a quadratic extension
`Q[sqrt(d)]`.

## Scope

The first Rust slice matches the implemented Python surface:

- Represent algebraic coefficients as `a + b*sqrt(d)` with exact rational
  `a` and `b`.
- Represent algebraic polynomials as coefficient lists in ascending degree.
- Split monic quadratics when the discriminant becomes a rational square after
  adjoining `sqrt(d)`.
- Split depressed monic quartics of the form `x^4 + p*x^2 + q` into conjugate
  quadratics when the Python pattern applies.
- Factor an integer polynomial over `Z` first through the existing Rust
  `cas-factor` package, then split each residual factor over `Q[sqrt(d)]`.
- Provide an IR adapter for `AlgFactor(poly, Sqrt(d))`-style callers without
  coupling this package to the symbolic VM.

## Non-goals

- General algebraic number fields beyond quadratic extensions.
- General polynomial factorization over algebraic extensions.
- VM registration. This crate exposes pure functions and IR conversion helpers;
  VM handler wiring remains a later integration slice.

## Validation

Rust unit tests cover:

- Rational square detection.
- `x^2 - d` splitting over `Q[sqrt(d)]`.
- `x^4 + 1` splitting over `Q[sqrt(2)]`.
- Non-splitting cases from the Python package.
- Keeping already rational factors when only a residual factor splits.
- `AlgFactor` IR conversion and malformed-input fallthrough.
