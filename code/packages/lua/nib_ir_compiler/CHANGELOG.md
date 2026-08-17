# Changelog

## 0.1.2

- Fixed: handle `shift_expr` in expression dispatch. #11257 inserted a new
  `shift_expr` precedence level between `add_expr` and `mul_expr`
  (`add_expr -> shift_expr -> mul_expr -> bitwise_expr -> unary_expr`) to
  support `<<`/`>>`, but only updated the Rust `nib-lexer`/`nib-parser`/
  `nib-iir-compiler`/`nib-type-checker` consumers. This compiler's
  `expression_rules` allow-list omitted `shift_expr`, so `expression_children`
  filtered both operands out of an enclosing `add_expr` -- every additive
  expression (even plain `a + b`) emitted no `ADD`/`ADD_IMM` at all, since
  `compile_add` saw zero operands. `shift_expr` is now in the rule table, so
  its (always single, since this Lua lexer does not yet tokenize `<<`/`>>`)
  child transparently passes through via the existing single-child recursion
  in `emit_expr_into`, exactly like `bitwise_expr`/`unary_expr` already do.
  Added a regression test for a plain two-operand `a + b`.

## 0.1.1

- Fixed: handle `mul_expr` in expression dispatch. LANG-FULL N1 added a
  multiplicative precedence level between `add_expr` and `bitwise_expr`;
  `mul_expr` was missing from `expression_rules`, so `expression_children`
  filtered it out and `compile_add` emitted no `ADD`/`ADD_IMM` for arithmetic
  such as `a +% b`. `mul_expr` is now in the rule table and routes through
  `compile_add`. Mirrors the Elixir fix (#5747).

## 0.1.0

- add the first Lua Nib IR compiler package
- lower the convergence-wave Nib subset into compiler IR
- emit loop and call shapes accepted by the existing Lua WASM backend
