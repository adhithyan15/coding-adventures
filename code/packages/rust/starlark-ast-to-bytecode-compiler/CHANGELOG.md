# Changelog

## 0.1.0 — 2026-03-22

### Added
- All ~30 Starlark grammar rule handlers ported from Python implementation
- Handlers for: `file`, `simple_stmt`, `suite`, `assign_stmt`, `return_stmt`,
  `break_stmt`, `continue_stmt`, `pass_stmt`, `load_stmt`, `if_stmt`,
  `for_stmt`, `def_stmt`, `expression`, `expression_list`, `or_expr`,
  `and_expr`, `not_expr`, `comparison`, `arith`, `term`, `shift`,
  `bitwise_or`, `bitwise_xor`, `bitwise_and`, `factor`, `power`,
  `primary`, `atom`, `list_expr`, `dict_expr`, `paren_expr`, `lambda_expr`
- Support for list/dict comprehensions with nested for/if clauses
- Function definitions with default parameters, *args, **kwargs
- Keyword argument passing with CALL_FUNCTION_KW
- Subscript and slice compilation
- Attribute access compilation
- Ternary expression compilation
- Short-circuit boolean evaluation (or/and)
- Augmented assignment operators (+=, -=, etc.)
- Tuple unpacking in assignments and for-loops
- Adjacent string literal concatenation at compile time
- `create_starlark_compiler()` factory function
- 70+ unit tests covering all handlers
- Literate programming style with extensive doc comments
