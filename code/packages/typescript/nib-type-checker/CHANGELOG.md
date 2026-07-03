# Changelog

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
