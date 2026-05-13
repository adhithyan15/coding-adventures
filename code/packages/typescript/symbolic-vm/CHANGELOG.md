# Changelog

## Unreleased

### Added

- Added a canonical symbolic `Factor` handler backed by `cas-factor`, including
  a small common-symbolic-factor extraction pass for multivariate expressions
  like `x^2*y - y`.
- Added a bivariate perfect-square factoring foothold so `Factor` recognises
  expressions like `x^2 + 2*x*y + y^2` as `(x+y)^2`.
- Added a bivariate difference-of-squares factoring foothold so `Factor`
  recognises expressions like `x^2 - y^2` as `(x-y)*(x+y)`.
- Added a symbolic-backend-only `D` handler for pure IR differentiation,
  including arithmetic, power, elementary, hyperbolic, and inverse hyperbolic
  chain rules.
- Added reciprocal hyperbolic `Coth`, `Sech`, and `Csch` numeric handlers and
  derivative chain rules expressed via `Sinh`/`Cosh`.

## [0.1.0] - 2026-05-08

### Added

- Initial pure TypeScript symbolic VM.
- Strict and symbolic backends.
- Arithmetic, elementary numeric, comparison, logic, assignment, definition,
  list, and user-function application handlers.
