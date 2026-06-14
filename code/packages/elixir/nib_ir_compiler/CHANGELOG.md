# Changelog

## [0.1.1] - 2026-06-14

### Fixed

- **`mul_expr` missing from IR-compiler expression dispatch** (latent bug from
  LANG-FULL N1): same root cause as the nib_type_checker fix — `mul_expr` was
  not in `expression_children/1` or `expression_rule?/1`, so an enclosing
  `add_expr` passed its `mul_expr` operands through `child_nodes` but
  `compile_add` called `expression_children` and got an empty list. Effect: any
  arithmetic body (`a +% b`) produced zero code-gen operands and emitted no
  `:add`/`:add_imm` opcodes. Fix: add `mul_expr` to both allow-lists and route
  `mul_expr` nodes through `compile_add` (additive and multiplicative
  expressions share the same binary-operand lowering logic in the current IR).

## [0.1.0]

- add the first Elixir Nib IR compiler package
- lower the convergence-wave Nib subset into compiler IR
- emit loop and call shapes accepted by the existing WASM backend
