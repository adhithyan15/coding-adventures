# Changelog

## 0.1.2

- fix: recognise the `shift_expr` precedence level. #11257 inserted
  `shift_expr` between `add_expr` and `mul_expr` (`add_expr → shift_expr →
  mul_expr → bitwise_expr`) to support `<<`/`>>`, but only updated the Rust
  consumers. This checker's `EXPRESSION_RULES` set omitted `shift_expr`, so
  `expressionChildren` filtered `shift_expr` operands out of an enclosing
  `add_expr` — every additive expression (even plain `a + b`) inferred no
  type. Added `shift_expr` alongside `mul_expr`/`bitwise_expr`; it already
  falls through `checkExpression`'s default single-child case, matching how
  `mul_expr` was fixed previously.

## 0.1.1

- fix: recognise the `mul_expr` precedence level (LANG-FULL N1). The grammar's
  cascade is `add_expr → mul_expr → bitwise_expr`, but the checker omitted
  `mul_expr` from its expression rules and dispatch. An enclosing `add_expr`
  therefore filtered its `mul_expr` operands out and inferred no type, so a body
  like `a +% b` (or `a * b`) annotated nothing. `mul_expr` now reuses the
  additive checker (same-type operands, numeric result, BCD restricted to
  `+%`/`-`). Mirrors the analogous nib-formatter fix (#5713).

## 0.1.0

- add a TypeScript port of the Nib type checker
