# Changelog

## 0.1.0 — 2026-04-10

### Added
- `DartmouthBasicParser.parse/1` — parse a pre-tokenized token list into an AST
- `DartmouthBasicParser.parse_source/1` — one-shot parse from raw BASIC source text
- `DartmouthBasicParser.create_parser/0` — load and return the ParserGrammar
- Grammar caching via `:persistent_term` for zero-copy repeated use
- Full support for all 17 Dartmouth BASIC 1964 statement types:
  LET, PRINT, INPUT, IF/THEN, GOTO, GOSUB, RETURN, FOR/NEXT, END, STOP,
  REM, READ, DATA, RESTORE, DIM, DEF
- Expression precedence cascade: expr → term → power → unary → primary
- Right-associative exponentiation (`^`)
- Built-in function calls (SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN)
- User-defined function calls (FNA–FNZ)
- Scalar and array variable access
- 47 unit tests covering all statement types, all relational operators,
  expression precedence/associativity, multi-line programs, error cases,
  and edge cases (bare line numbers, empty programs)
