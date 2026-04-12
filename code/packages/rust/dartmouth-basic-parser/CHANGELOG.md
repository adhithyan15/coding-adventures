# Changelog — coding-adventures-dartmouth-basic-parser

All notable changes to this crate will be documented in this file.

## [0.1.0] — 2026-04-10

### Added

- Initial implementation of the Dartmouth BASIC parser.
- `parse_dartmouth_basic(source: &str) -> GrammarASTNode` — one-call entry
  point that tokenizes the source and parses it into an AST with root rule
  `"program"`.
- `create_dartmouth_basic_parser(source: &str) -> GrammarParser` — factory
  function that returns a configured `GrammarParser` for callers that need
  fine-grained control over the parse step.
- Grammar path resolution via `env!("CARGO_MANIFEST_DIR")` pointing to
  `code/grammars/dartmouth_basic.grammar`.
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
  exponentiation (right-associative), unary minus, parentheses.
- All 11 built-in function tests: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR,
  INT, RND, SGN.
- User-defined function tests: FNA, FNZ.
- Array subscript in expressions: A(I), A(I+1).
- Multi-line program tests: hello world, counting loop, conditional,
  subroutine.
- Edge case test: bare line number `"10\n"` is valid BASIC.
- Factory function test: `create_dartmouth_basic_parser` returns a working
  parser.
- READ/DATA round-trip program test.
- Complex expression precedence test.
- Literate programming style with detailed inline comments explaining
  1964 Dartmouth BASIC history and the grammar-driven parser approach.
