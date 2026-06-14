# Changelog

## [0.1.1] - 2026-06-14

### Fixed

- **`mul_expr` missing from expression dispatch** (latent bug from LANG-FULL N1):
  the grammar gained a multiplicative precedence level (`mul_expr`) between
  `add_expr` and `bitwise_expr`. The type checker's `@expression_rules`
  allow-list was never updated, so `expression_children/1` silently filtered out
  `mul_expr` nodes whenever `add_expr` walked its operands. Effect: any function
  body containing arithmetic (`a +% b`) or a call expression nested under an
  `add_expr` cascade inferred type `nil`, causing spurious "return expects T,
  got nil" errors on valid code. Fix: add `mul_expr` to `@expression_rules` and
  add a dedicated `check_expr` clause for `mul_expr` (same numeric inference
  semantics as `add_expr`). Regression tests added for `*` expressions and
  operand-type mismatches.

## [0.1.0]

- add the first Elixir Nib type checker package
- validate the convergence-wave Nib subset used by the local WASM lane
- return a typed AST wrapper with per-node type metadata
