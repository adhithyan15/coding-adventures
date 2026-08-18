# Changelog

## 0.1.1 — 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_parser/1` now returns the
  pre-compiled `CodingAdventures.AlgolParser.Grammar.Algol60` module
  instead of `File.read!`-ing `algol60.grammar` from `code/grammars/` on
  every call. The old code walked out of the installed package's own
  directory to a monorepo-relative path that a published Hex package does
  not ship, so `mix deps.get` + first use would raise a `File.Error`. This
  required fixing a gap in the shared `grammar_tools` compiler:
  `algol60.grammar`'s `&(SEMICOLON)`-style positive lookahead previously
  crashed `compile-grammar` (now handled, see `grammar_tools`'s own
  CHANGELOG).

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
