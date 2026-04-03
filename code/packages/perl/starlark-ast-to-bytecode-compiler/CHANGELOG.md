# Changelog — CodingAdventures::StarlarkAstToBytecodeCompiler

## 0.01 — 2026-03-31

### Added

- Initial Perl port of the Starlark AST-to-bytecode compiler (Elixir reference:
  `elixir/starlark_ast_to_bytecode_compiler`).
- Full opcode constants: 46 opcodes matching the Elixir reference.
- Operator-to-opcode hashrefs: BINARY_OP_MAP, COMPARE_OP_MAP, AUGMENTED_ASSIGN_MAP,
  UNARY_OP_MAP (exported as class methods).
- Compiler class with register_rule(), compile_node(), emit(), emit_jump(),
  patch_jump(), add_constant(), add_name().
- Handlers for all grammar rules: file, suite, statement, simple_stmt, small_stmt,
  compound_stmt, expression_stmt, assign_stmt, augmented_assign_stmt, return_stmt,
  pass_stmt, break_stmt, continue_stmt, if_stmt, elif_clause, else_clause, for_stmt,
  load_stmt, def_stmt, param_list, param, call, call_args, argument, dot_access,
  subscript, slice, expr, expression, or_expr, and_expr, not_expr, comparison,
  arith, term, shift, bitwise_and, bitwise_xor, bitwise_or, factor, unary,
  power_expr, primary, atom, identifier, number, string_node, list_expr,
  dict_expr, dict_entry, tuple_expr, lambda_expr, list_comp, dict_comp,
  comp_clause, comp_if, star_expr.
- compile_ast() class method for one-shot compilation.
- token_node() and ast_node() constructors for building test ASTs.
- 95%+ test coverage with Test2::V0.
