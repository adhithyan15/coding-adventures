# Changelog — coding-adventures-starlark-ast-to-bytecode-compiler (Lua)

## 0.1.0 — 2026-03-31

### Added

- Initial Lua port of the Starlark AST-to-bytecode compiler (Elixir reference:
  `elixir/starlark_ast_to_bytecode_compiler`).
- Full opcode constants: 46 opcodes spanning stack ops, variable ops, arithmetic,
  bitwise, comparison, boolean, control flow, functions, collections, subscript,
  iteration, module, I/O, and VM control.
- Operator-to-opcode maps: BINARY_OP_MAP, COMPARE_OP_MAP, AUGMENTED_ASSIGN_MAP,
  UNARY_OP_MAP.
- StarlarkCompiler wrapping GenericCompiler with handlers for all grammar rules:
  file, suite, statement, simple_stmt, small_stmt, compound_stmt, expression_stmt,
  assign_stmt, augmented_assign_stmt, return_stmt, pass_stmt, break_stmt,
  continue_stmt, if_stmt, elif_clause, else_clause, for_stmt, load_stmt, def_stmt,
  param_list, param, call, call_args, argument, dot_access, subscript, slice,
  expr, expression, or_expr, and_expr, not_expr, comparison, arith, term, shift,
  bitwise_and, bitwise_xor, bitwise_or, factor, unary, power_expr, primary, atom,
  identifier, number, string_node, list_expr, dict_expr, dict_entry, tuple_expr,
  lambda_expr, list_comp, dict_comp, comp_clause, comp_if, star_expr.
- compile_ast() top-level convenience function.
- Helper constructors: token_node(), ast_node() for building ASTs in tests.
- 95%+ test coverage with busted.
