# TypeScript CAS Limit/Series Parity

## Status

Initial parity slice for the Rust `cas-limit-series` package.

## Scope

This TypeScript package intentionally ports the behavior that is currently
implemented in Rust:

- Direct finite limits by structural substitution.
- Unevaluated `Limit(expr, var, point)` for the literal indeterminate
  `Div(0, 0)` after substitution.
- Polynomial-only Taylor expansion around integer, rational, or float literal
  points.
- Exact rational coefficient arithmetic using `bigint`.

## Non-goals for this slice

- L'Hopital-based limit resolution.
- Transcendental Taylor expansions.
- Laurent or Puiseux series.
- Multivariate limits.

Those remain future extensions described by `cas-limit-series.md`.
