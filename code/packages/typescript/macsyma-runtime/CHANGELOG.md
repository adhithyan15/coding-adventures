# Changelog

## Unreleased

- Wire `Solve(List(...), List(...))` / `linsolve` to the TypeScript
  `cas-solve` exact linear-system solver, returning `List(Rule(var, value))`
  for square systems and preserving unevaluated fallback behavior.

## 0.1.0

- Add pure TypeScript MACSYMA runtime sessions, history lookup, display
  metadata, constants, and JSON result helpers.
