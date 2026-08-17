# Changelog

## 0.1.2

- Fixed: recognise the `shift_expr` precedence level. Same root cause as the
  nib_type_checker fix — #11257 inserted `shift_expr` between `add_expr` and
  `mul_expr` (`add_expr → shift_expr → mul_expr → bitwise_expr`) to support
  `<<`/`>>`, but only updated the Rust consumers. Ruby's `nib_parser` reads
  the shared `nib.grammar` file at runtime, so it started wrapping every
  `add_expr` operand in a `shift_expr` node this compiler's
  `expression_rule?` didn't recognise. That broke `compile_add`'s operand
  lookup (it falls back to `expression_rule?` when the child isn't in its
  inline rule-name list) and `emit_expr_into`'s generic single-child
  passthrough, so no `ADD`/`ADD_IMM` was ever emitted for additive
  expressions, even plain `a + b`. Added `shift_expr` alongside
  `mul_expr`/`bitwise_expr`; since Ruby's `nib_lexer` never tokenizes
  `SHL`/`SHR`, every `shift_expr` node has exactly one child, so (like
  `mul_expr` before it) it needs no dedicated lowering branch next to
  `compile_add`. (Note: this release also folds in the version bump that
  #7378's `mul_expr` fix below should have shipped with but didn't —
  `lib/.../version.rb` was left at `0.1.0`.)

## 0.1.1

- Fixed: handle `mul_expr` in expression dispatch. Same root cause as the
  nib_type_checker fix — LANG-FULL N1 added a multiplicative precedence level
  between `add_expr` and `bitwise_expr`, and `mul_expr` was missing from
  `expression_rule?`, so `compile_add` saw no operands and emitted no
  `ADD`/`ADD_IMM` for arithmetic like `a +% b`. `mul_expr` now routes through
  `compile_add`. Mirrors the Elixir fix (#5747).

## 0.1.0

- add the first Ruby Nib IR compiler package
- lower the convergence-wave Nib subset into generic compiler IR
- emit loop and call shapes compatible with the existing Ruby WASM backend
