# Changelog

## 0.1.0 — 2026-04-10

### Added
- `ParseDartmouthBasic(source string)` — one-shot parse from BASIC source to AST
- `CreateDartmouthBasicParser(source string)` — factory returning a configured GrammarParser
- Capability-cage enforcement via `gen_capabilities.go` — only `dartmouth_basic.grammar` may be read
- Full support for all 17 Dartmouth BASIC 1964 statement types:
  LET, PRINT, INPUT, IF/THEN, GOTO, GOSUB, RETURN, FOR/NEXT, END, STOP,
  REM, READ, DATA, RESTORE, DIM, DEF
- Expression precedence cascade: expr → term → power → unary → primary
- Right-associative exponentiation (`^`)
- Built-in function calls (SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN)
- User-defined function calls (FNA–FNZ)
- Scalar and array variable access
- 48 unit tests covering all statement types, all relational operators,
  expression precedence/associativity, multi-line programs, error cases,
  and edge cases (bare line numbers, empty programs)
