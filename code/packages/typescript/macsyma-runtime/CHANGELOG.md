# Changelog

## Unreleased

- Wire `assume`, `forget`, `is`, `declare`, `properties`, and `propvars` to a
  TypeScript MACSYMA session assumption context so declared properties feed
  property queries and assumption-backed relation checks.
- Route `ev(expr, display2d)` JSON and browser display text through the
  TypeScript `cas-pretty-printer` 2D MACSYMA box renderer while preserving the
  symbolic IR result for history and downstream evaluation.
- Wire `TrigReduce` runtime dispatch to the TypeScript `cas-trig` package.
- Wire `Simplify`, `RatSimplify`, `TrigSimplify`, and `TrigExpand` runtime
  heads to the TypeScript `cas-simplify` and `cas-trig` packages.
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
