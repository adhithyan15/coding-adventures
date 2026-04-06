# Changelog

## 0.1.0 — 2026-04-06

### Added
- `AlgolParser.parse/1` — parse ALGOL 60 source code into an AST
- `AlgolParser.create_parser/0` — parse the `algol.grammar` file and return the `ParserGrammar`
- Grammar caching via `persistent_term` for fast repeated calls
- 45 tests covering:
  - Grammar inspection (`create_parser/0`): top-level, declaration, statement, and expression rules
  - Minimal programs: `begin end`, `begin integer x; x := 42 end`
  - Declarations: `integer`, `real`, `boolean`, multiple variables in one declaration
  - Assignment: integer, real, and expression right-hand sides
  - Arithmetic expressions: `+`, `-`, `*`, `/`, `div`, `mod`, `**` (exponentiation), parenthesized expressions
  - Conditional statements: `if/then`, `if/then/else`, relational operators (`<=`, `=`)
  - Boolean expressions: `and`, `or`, `not`, boolean literals `true`/`false`
  - For loops: step/until form, while form, simple value form
  - Nested blocks: multiple `begin/end` levels, compound statements
  - Procedure calls: with arguments, with no arguments
  - Goto statements
  - String literals in programs
  - Multiple statements separated by semicolons
  - Comment handling (lexer/parser integration)
  - `ASTNode` helpers
  - Error cases: unclosed block, unexpected character, declaration without statement
