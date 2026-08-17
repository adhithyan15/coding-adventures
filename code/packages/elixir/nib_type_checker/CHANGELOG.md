# Changelog

## [0.1.2] - 2026-08-17

### Fixed

- **`shift_expr` missing from expression dispatch** (same root cause as the
  `mul_expr` fix in 0.1.1, one precedence level up): #11257 inserted a new
  `shift_expr` level between `add_expr` and `mul_expr`
  (`add_expr → shift_expr → mul_expr → bitwise_expr`) to support `<<`/`>>`,
  but only updated the Rust consumers (`nib-lexer`, `nib-parser`,
  `nib-iir-compiler`, `nib-type-checker`). Elixir's `nib_parser` loads the
  shared `nib.grammar` file at runtime, so it immediately started emitting a
  `shift_expr` wrapper node around every `mul_expr` operand of `add_expr` —
  including ordinary `a + b`. This checker's `@expression_rules` allow-list
  was never updated, so `expression_children/1` filtered `shift_expr` nodes
  out of `add_expr`'s operands, and `add_expr` inferred type `nil` for any
  additive expression, cascading into spurious "return expects T, got nil"
  errors on valid code. Fix: add `shift_expr` to `@expression_rules`. No new
  `check_expr` clause is needed — Elixir's `nib_lexer` does not tokenize
  `SHL`/`SHR` yet, so every `shift_expr` node the parser produces wraps
  exactly one `mul_expr` child, and the existing generic single-child
  `check_expr` clause already passes such wrapper nodes through correctly
  (same mechanism the mul_expr fix in 0.1.1 established). Regression test
  added for a plain two-operand `a + b` inside a function body.

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
