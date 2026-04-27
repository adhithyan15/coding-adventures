# Changelog

## [0.1.0] - 2026-04-20

### Added

- Initial implementation of the Oct parser — a thin wrapper around the generic
  `GrammarParser` that loads `code/grammars/oct.grammar` and tokenizes input
  with the `oct-lexer` package.
- `create_oct_parser(source)` — creates a `GrammarParser` configured for Oct,
  returning a parser instance whose `.parse()` method produces an `ASTNode` tree.
- `parse_oct(source)` — the main entry point; tokenizes and parses Oct source
  text in one call, returning the root `ASTNode` (rule name: `"program"`).
- `OCT_GRAMMAR_PATH` — the path constant pointing at `code/grammars/oct.grammar`,
  resolved relative to the module file.
- Grammar rules in `code/grammars/oct.grammar` (committed alongside this package):
  - Entry point: `program = { top_decl }`
  - Top-level: `static_decl`, `fn_decl`
  - Statements: `let_stmt`, `assign_stmt`, `return_stmt` (optional expr for void
    functions), `if_stmt` (optional else), `while_stmt`, `loop_stmt`,
    `break_stmt`, `expr_stmt`
  - Expressions (8 precedence levels): `or_expr`, `and_expr`, `eq_expr`,
    `cmp_expr`, `add_expr`, `bitwise_expr`, `unary_expr`, `primary`
  - Intrinsic calls: `intrinsic_call` covering all 10 Oct intrinsics (`in`,
    `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`)
  - User-defined calls: `call_expr = NAME LPAREN [ arg_list ] RPAREN`
- Comprehensive test suite (`tests/test_oct_parser.py`):
  - `TestParseOctBasic` — root node rule name, empty program, single static,
    single function
  - `TestStaticDeclarations` — u8 and bool statics, hex/binary/decimal values,
    multiple statics
  - `TestFunctionDeclarations` — void no-params, return type, with params,
    multiple params
  - `TestStatements` — let, assign, return (with and without expr), if (with
    and without else), while, loop, break, expr_stmt
  - `TestExpressions` — all binary operators, unary operators, operator
    precedence, parenthesised expressions
  - `TestIntrinsicCalls` — all 10 intrinsics parse without error and produce
    `intrinsic_call` nodes
  - `TestCompletePrograms` — all five OCT00 spec examples parse successfully
    and produce the correct top-level structure
