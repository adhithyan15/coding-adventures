# Changelog

## Unreleased

- Wire `Subst(value, variable, expr)` to the TypeScript `cas-substitution`
  structural substitution package.
- Wire deterministic MACSYMA list heads (`Length`, `First`, `Rest`, `Last`,
  `Append`, `Reverse`, `Range`, `Map`, `Apply`, `Sort`, `Part`, `Flatten`,
  and `Join`) to the TypeScript `cas-list-operations` package.
- Wire direct `Solve(f(linear) = constant, var)` transcendental equations to
  the TypeScript `cas-solve` `trySolveTranscendental` handler, returning
  `List(...)` symbolic inverse and periodic-family solutions.
- Wire `Solve(inequality, var)` to the TypeScript `cas-solve`
  `trySolveInequality` handler for polynomial inequalities, returning
  `List(...)` interval predicates and preserving unevaluated fallback behavior.
- Wire `Solve(List(...), List(...))` / `linsolve` to the TypeScript
  `cas-solve` exact linear-system solver, returning `List(Rule(var, value))`
  for square systems and preserving unevaluated fallback behavior.

## 0.1.0

- Add pure TypeScript MACSYMA runtime sessions, history lookup, display
  metadata, constants, and JSON result helpers.
