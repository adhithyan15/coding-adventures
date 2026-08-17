# Changelog

## 0.1.2

- Fixed: recognise the `shift_expr` precedence level. #11257 inserted
  `shift_expr` between `add_expr` and `mul_expr` (`add_expr → shift_expr →
  mul_expr → bitwise_expr`) to support `<<`/`>>`, but only updated the Rust
  consumers. Ruby's `nib_parser` reads the shared `nib.grammar` file at
  runtime rather than a generated copy, so it started wrapping every
  `add_expr` operand in a `shift_expr` node this checker's
  `expression_rule?` didn't recognise — `expression_children` filtered it
  out, so `check_add_expr` saw zero operands and inferred no type for any
  additive expression, even plain `a + b`. Added `shift_expr` alongside
  `mul_expr`/`bitwise_expr`; since Ruby's `nib_lexer` never tokenizes
  `SHL`/`SHR`, every `shift_expr` node has exactly one child, so (like
  `mul_expr` before it) it falls through `check_expr`'s default single-child
  case without needing its own dispatch branch. (Note: this release also
  folds in the version bump that #7378's `mul_expr` fix below should have
  shipped with but didn't — `lib/.../version.rb` was left at `0.1.0`.)

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
