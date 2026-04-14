# Changelog — CodingAdventures::StarlarkParser

## 0.01 — initial release

- Implement `CodingAdventures::StarlarkParser` — hand-written recursive-descent
  Starlark parser that tokenizes with `StarlarkLexer` and builds an AST.
- Implement `CodingAdventures::StarlarkParser::ASTNode` — lightweight AST node
  class with leaf/inner distinction matching the JavascriptParser pattern.
- Root AST node has `rule_name == "program"`.
- Convenience class method `parse_starlark($source)` — tokenize and parse in one call.

Grammar rules implemented:
  - `_parse_program` — top-level file (sequence of statements)
  - `_parse_statement` — dispatch to compound or simple
  - `_parse_simple_stmt` — semicolon-separated small statements + NEWLINE
  - `_parse_small_stmt` — dispatch to return/break/continue/pass/load/assign
  - `_parse_return_stmt` — `return [expr]`
  - `_parse_load_stmt` — `load("module", "sym", alias = "sym")`
  - `_parse_assign_stmt` — assignment, augmented assignment, expression statement
  - `_parse_if_stmt` — if/elif/else with arbitrary elif chain
  - `_parse_for_stmt` — for loop (no while)
  - `_parse_loop_vars` — single NAME or tuple of NAMEs
  - `_parse_def_stmt` — function definition
  - `_parse_suite` — indented block or inline simple_stmt
  - `_parse_parameters` / `_parse_parameter` — def parameter list
  - `_parse_expression_list` — comma-separated expressions for assignment/tuples
  - `_parse_expression` — lambda or conditional (ternary)
  - `_parse_lambda_expr` / `_parse_lambda_params` / `_parse_lambda_param`
  - `_parse_or_expr`, `_parse_and_expr`, `_parse_not_expr` — boolean operators
  - `_parse_comparison` — ==, !=, <, >, <=, >=, in, not in
  - `_parse_bitwise_or`, `_parse_bitwise_xor`, `_parse_bitwise_and`
  - `_parse_shift` — << and >>
  - `_parse_arith` — + and -
  - `_parse_term` — *, /, //, %
  - `_parse_factor` — unary +, -, ~
  - `_parse_power` — ** (right-associative)
  - `_parse_primary` — atom with suffixes (.attr, [subscript], (args))
  - `_parse_subscript` — index or slice
  - `_parse_atom` — INT, FLOAT, STRING, NAME, True, False, None, list, dict, paren
  - `_parse_list_expr` — list literal or comprehension
  - `_parse_dict_expr` — dict literal or comprehension
  - `_parse_paren_expr` — parenthesized expression or tuple
  - `_parse_comp_clause`, `_parse_comp_for`, `_parse_comp_if` — comprehensions
  - `_parse_arguments` / `_parse_argument` — call arguments (positional, keyword, *splat, **splat)

Test suite (Test2::V0):
  - `t/00-load.t` — module loading and VERSION check
  - `t/01-basic.t` — comprehensive tests:
    - ASTNode inner/leaf node unit tests
    - Root node and empty program
    - Simple, augmented, and tuple-unpacking assignments
    - Function calls, load statements, BUILD-style rules
    - Function definitions (no params, default params, multi-statement bodies)
    - List/dict literals
    - If/elif/else and for loops
    - Break, continue, pass statements
    - Expressions (arithmetic, comparison, boolean, ternary, lambda, attributes)
    - Complex programs (def + if + return, BUILD file with load)
    - Error handling (garbage input dies)
