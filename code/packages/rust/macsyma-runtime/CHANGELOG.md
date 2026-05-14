# Changelog

## Unreleased

### Added

- Delegated Rust MACSYMA `factor` evaluation to the shared `symbolic-vm`
  canonical handler, including coverage for common multivariate factors.
- Added Rust MACSYMA parity for bivariate perfect-square factoring, so
  `factor(x^2 + 2*x*y + y^2)` returns `(x + y)^2` through the shared symbolic
  VM handler.
- Added Rust MACSYMA parity for bivariate difference-of-squares factoring, so
  `factor(x^2 - y^2)` returns `(x - y) * (x + y)` through the shared symbolic
  VM handler.
- Added Rust MACSYMA parity for bivariate cubic-identity factoring, so
  `factor(x^3 - y^3)` and `factor(x^3 + y^3)` return their linear/quadratic
  products through the shared symbolic VM handler.
- Added Rust MACSYMA parity for four-term bilinear grouping, so
  `factor(x*y + x*z + y + z)` returns `(x + 1) * (y + z)` through the shared
  symbolic VM handler.
- Added Rust MACSYMA parity for shared multivariate integer content, so
  `factor(2*x*y + 2*x*z)` returns `2*x*(y + z)` through the shared symbolic VM
  handler.
- Added Rust MACSYMA parity for four-term perfect-cube expansions, so
  `factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3)` returns `(x + y)^3` and
  `factor(x^3 - 3*x^2*y + 3*x*y^2 - y^3)` returns `(x - y)^3` through the
  shared symbolic VM handler.
- Added `?` / `? topic` help-query handling for Rust MACSYMA sessions.
- Wired `assume`, `forget`, `is`, `declare`, `properties`, and `propvars` to a
  Rust MACSYMA session assumption context so declared properties feed property
  queries and assumption-backed relation checks.

## [0.1.1] - 2026-05-12

### Added

- Routed `ev(expr, display2d)` result presentation through the Rust
  `cas-pretty-printer` 2D MACSYMA box renderer while preserving the symbolic IR
  result for history and downstream evaluation.
- Wired `TrigReduce(expr)` and `ev(expr, trigreduce)` to the Rust `cas-trig`
  power-reduction walker.
- Wired `Subst(value, variable, expr)` to the Rust `cas-substitution`
  structural substitution package.
- Wired deterministic MACSYMA list heads (`Length`, `First`, `Rest`, `Last`,
  `Append`, `Join`, `Reverse`, `Range`, `Part`, `Map`, `Apply`, `Sort`, and
  `Flatten`) to the Rust `cas-list-operations` package.
- Wired direct `Solve(f(linear) = constant, variable)` transcendental equations
  to the Rust `cas-solve` `try_solve_transcendental` handler, returning
  `List(...)` symbolic inverse and periodic-family solutions.
- Wired `Solve(inequality, variable)` to the Rust `cas-solve`
  `try_solve_inequality` handler, returning `List(...)` interval predicates
  for supported one-variable polynomial inequalities.
- Wired `linsolve` / `Solve(List(...), List(...))` to the Rust `cas-solve`
  exact linear-system solver.
- Added runtime coverage for integer systems, rational systems, and non-linear
  fallback.

## [0.1.0] - 2026-05-08

### Added

- Initial Rust MACSYMA runtime session facade.
- Source-to-IR compilation, symbolic VM evaluation, display/suppress result metadata, and in-memory input/output history.
