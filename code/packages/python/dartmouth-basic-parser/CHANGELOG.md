# Changelog — coding-adventures-dartmouth-basic-parser

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-10

### Added

- Initial release of the Dartmouth BASIC 1964 parser.
- `parse_dartmouth_basic(source: str) -> ASTNode` — parses a BASIC program string
  into a generic AST. The root node always has `rule_name="program"`.
- `create_dartmouth_basic_parser(source: str) -> GrammarParser` — factory that
  tokenizes BASIC source and creates a `GrammarParser` configured with
  `dartmouth_basic.grammar`, ready to call `.parse()` on.
- Full grammar-driven parsing of all 17 statement types in the 1964 spec:
  `LET`, `PRINT`, `INPUT`, `IF...THEN`, `GOTO`, `GOSUB`, `RETURN`,
  `FOR...NEXT`, `END`, `STOP`, `REM`, `READ`, `DATA`, `RESTORE`, `DIM`, `DEF`.
- Expression precedence encoded by grammar rule nesting (no special annotations):
  addition/subtraction → multiplication/division → exponentiation → unary → primary.
- Right-associative exponentiation (`2 ^ 3 ^ 2` = `2 ^ (3 ^ 2)` = 512).
- Support for all 11 built-in functions: SIN, COS, TAN, ATN, EXP, LOG, ABS,
  SQR, INT, RND, SGN.
- User-defined function calls (FNA–FNZ style).
- Array subscript access in both read and write positions.
- Comprehensive test suite covering all statement types, expression precedence,
  multi-line programs, bare line numbers, and error cases.
- Literate inline documentation explaining 1964 Dartmouth BASIC history,
  the grammar-driven parsing approach, and how to read the AST output.
