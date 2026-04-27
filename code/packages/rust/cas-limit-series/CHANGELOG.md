# Changelog — cas-limit-series (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-limit-series` package.
- `limit` module: `limit_direct(expr, var, point) -> IRNode`.
  - Direct-substitution limit via `cas_substitution::subst`.
  - Conservative indeterminate-form detection: `Div(0, 0)` → unevaluated
    `Limit(expr, var, point)`.
  - Does not simplify; caller passes result through `cas_simplify::simplify`.
- `taylor` module: `taylor_polynomial(expr, var, point, order) -> Result<IRNode, PolynomialError>`.
  - Polynomial Taylor expansion using exact rational coefficient arithmetic.
  - Internal `Frac { numer: i128, denom: i128 }` type with full arithmetic.
  - `to_coefficients` — IR → `Vec<Frac>` coefficient list for `Add`, `Sub`,
    `Neg`, `Mul`, `Pow` (non-negative integer exponents), `Div` (constant
    denominator), numeric literals, and the expansion variable.
  - `shift_polynomial` — polynomial shift via falling-factorial formula.
  - `from_coefficients` — `Vec<Frac>` + variable + point → IR tree.
  - `PolynomialError` error type; raised on transcendental or multi-variable
    inputs.
- Head-name string constants: `LIMIT`, `TAYLOR`, `SERIES`, `BIG_O`.
- 16 integration tests + 3 doc-tests; all passing.
