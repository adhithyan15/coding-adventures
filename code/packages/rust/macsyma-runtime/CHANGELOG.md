# Changelog

## [0.1.1] - 2026-05-12

### Added

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
