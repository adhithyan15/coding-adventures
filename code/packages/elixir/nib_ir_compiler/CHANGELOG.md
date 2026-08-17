# Changelog

## [0.1.2] - 2026-08-17

### Fixed

- **`shift_expr` missing from IR-compiler expression dispatch** (same root
  cause as the nib_type_checker 0.1.2 fix and the mul_expr fix in 0.1.1):
  #11257 inserted a new `shift_expr` precedence level between `add_expr` and
  `mul_expr`. Elixir's `nib_parser` (which loads the shared `nib.grammar`
  file at runtime) started wrapping every `add_expr` operand in a
  `shift_expr` node, including for plain `a + b`. This package keeps its own
  copy of the expression rule-name allow-list (`expression_children/1` and
  `expression_rule?/1`), and it didn't know about `shift_expr` either, so
  `compile_add` — called directly for `add_expr` — received an empty operand
  list and silently emitted no `:add`/`:add_imm`/`:sub` instructions for any
  additive expression. Fix: hoist the allow-list into a module attribute
  (`@expression_rules`, mirroring nib_type_checker) and add `shift_expr` to
  it. `emit_expr_into/3`'s existing single-child unwrap branch
  (`expression_rule?(node.rule_name) and length(child_nodes(node)) == 1`)
  already passes such wrapper nodes through to their `mul_expr` child
  correctly — no new codegen case needed, and no real `<<`/`>>` shift
  semantics were added (Elixir's `nib_lexer` doesn't tokenize `SHL`/`SHR`
  yet). Regression test added for a plain two-operand `a + b` add.

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
