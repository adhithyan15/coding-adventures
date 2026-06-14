# Changelog — cas-limit-series (Rust)

## [0.3.0] - 2026-05-29

### Added

- Track J2: `series_limit` module porting Python Track J1 (PR #5574). New
  `try_series_limit` / `try_series_limit_default` resolve transcendental
  `0/0` limits via a self-contained rational-coefficient series ring
  with bounded order (4 → 6 → 8 → 10 → 12). Wired into `limit_advanced`
  after the L'Hopital path and before the unevaluated `Limit(...)`
  fallthrough. Closes the canonical acceptance set:
  `limit((sin(x) - x)/x^3, x, 0) = -1/6`,
  `limit((1 - cos(x))/x^2, x, 0) = 1/2`,
  `limit((exp(x) - 1 - x)/x^2, x, 0) = 1/2`,
  `limit((tan(x) - x)/x^3, x, 0) = 1/3`,
  `limit((log(1 + x) - x)/x^2, x, 0) = -1/2`.
- `limit_advanced(sin(x)/x, x, 0)` without a `diff_fn` now closes to `1`.

## [0.2.2] - 2026-06-06

### Added

- Add bounded-over-diverging limit recognition at infinity, closing
  `limit(sin(x)/x, x, inf)` and `limit(cos(x)/(x^2+1), x, minf)` to exact
  `0` instead of returning an unevaluated `Limit(...)`.

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
