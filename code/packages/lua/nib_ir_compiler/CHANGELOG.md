# Changelog

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
