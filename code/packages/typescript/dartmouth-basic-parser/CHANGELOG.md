# Changelog — @coding-adventures/dartmouth-basic-parser

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-10

### Added

- Initial implementation of the Dartmouth BASIC parser.
- `parseDartmouthBasic(source: string): ASTNode` — one-call entry point
  that tokenizes the source and parses it into an AST with root rule
  `"program"`.
- Grammar path resolution via `__dirname` (4 levels up from `src/`) pointing
  to `code/grammars/dartmouth_basic.grammar`.
- ESM module with `"type": "module"` in package.json.
- Complete test suite covering all 17 statement types:
  - LET (scalar and array element assignment)
  - PRINT (bare, expression, string, comma separator, semicolon separator)
  - INPUT (single and multiple variables)
  - IF-THEN (all 6 relational operators: =, <, >, <=, >=, <>)
  - GOTO
  - GOSUB / RETURN
  - FOR / NEXT (with and without STEP)
  - END / STOP
  - REM
  - READ / DATA / RESTORE
  - DIM (single and multiple declarations)
  - DEF (user-defined function)
- Expression tests: addition, subtraction, multiplication, division,
  exponentiation (right-associative), unary minus, parentheses,
  complex multi-operator expressions.
- All 11 built-in function tests: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR,
  INT, RND, SGN.
- User-defined function tests: FNA, FNZ.
- Array subscript in expressions: A(I), A(I+1).
- Multi-line program tests: hello world, counting loop, conditional,
  subroutine.
- Edge case test: bare line number `"10\n"` is valid BASIC.
- Literate programming style with detailed inline comments explaining
  1964 Dartmouth BASIC history and the grammar-driven parser approach.
- Vitest test suite with `@vitest/coverage-v8` coverage reporting.
- Dependencies on `@coding-adventures/grammar-tools`, `@coding-adventures/parser`,
  `@coding-adventures/dartmouth-basic-lexer`, `@coding-adventures/lexer`,
  `@coding-adventures/directed-graph` as `file:` references.
