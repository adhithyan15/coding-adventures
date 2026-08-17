# Changelog

## [0.1.2] - 2026-08-17

### Fixed

- Handle `shift_expr` in the expression-rule allow-list. #11257 inserted a
  shift precedence level between `add_expr` and `mul_expr`
  (`add_expr → shift_expr → mul_expr → bitwise_expr`) to prepare for `<<`/`>>`,
  but only updated the Rust consumers. `shift_expr` was missing from
  `%_EXPRESSION_RULE`, so `_expression_children` filtered it out when
  `add_expr` walked its operands — the same mechanism as the `mul_expr` gap
  fixed in 0.1.1. Every additive expression, even plain `a + b`, then inferred
  `undef`, which slipped past compatibility checks — so an invalid
  `let x: bool = 1 + 2;` was accepted instead of raising a type error. Added
  `shift_expr` alongside `mul_expr`/`bitwise_expr`; unlike `mul_expr` it does
  not need its own `_check_add_expression` dispatch case, because the Nib
  lexer does not yet tokenize `SHL`/`SHR`, so every `shift_expr` node the
  parser produces wraps exactly one `mul_expr` child, and that single-child
  case already falls through `_check_expression`'s generic pass-through
  branch correctly.

## [0.1.1] - 2026-07-02

### Fixed

- Handle `mul_expr` in expression dispatch. LANG-FULL N1 inserted a
  multiplicative precedence level (`add_expr → mul_expr → bitwise_expr`), but
  `mul_expr` was missing from `%_EXPRESSION_RULE`, so `_expression_children`
  filtered it out when `add_expr` walked its operands. Arithmetic like
  `1 +% 2` then inferred `undef`, which slipped past compatibility checks — so
  an invalid `let x: bool = 1 +% 2;` was accepted instead of raising a
  type error. `mul_expr` now routes through `_check_add_expression` (shared
  numeric semantics). Mirrors the Elixir/Ruby/Lua fixes (#5747).

## [0.1.0] - 2026-04-18

### Added

- Semantic checking for variables, returns, loops, calls, and boolean
  expressions in Perl's Nib frontend.
- `check_source()` and `check()` helpers that return the shared
  `type-checker-protocol` result shape.
- Coverage for successful programs, assignment mismatches, and parse failures.
