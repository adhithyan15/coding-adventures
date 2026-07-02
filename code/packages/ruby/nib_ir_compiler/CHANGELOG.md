# Changelog

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
