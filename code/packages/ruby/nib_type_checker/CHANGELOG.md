# Changelog

## 0.1.1

- Fixed: handle `mul_expr` in expression dispatch. LANG-FULL N1 inserted a
  multiplicative precedence level (`add_expr → mul_expr → bitwise_expr`), but
  `mul_expr` was missing from `expression_rule?`, so `expression_children`
  filtered it out when `add_expr` walked its operands. Any arithmetic (`a +% b`),
  call, or literal under the cascade then inferred `nil`, producing spurious
  "return/let expects T, got " errors. `mul_expr` now routes through the
  additive checker (shared numeric semantics). Mirrors the Elixir/TypeScript
  fix (#5747).

## 0.1.0

- add the first Ruby Nib type checker package
- validate the convergence-wave Nib subset used by the WASM smoke tests
- return typed AST wrappers compatible with the Ruby Nib IR compiler
