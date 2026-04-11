# Changelog — coding-adventures-dartmouth-basic-parser (Lua)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-10

### Added

- Initial implementation of the grammar-driven Dartmouth BASIC parser.
- `M.parse(source)` — tokenizes BASIC source via `dartmouth_basic_lexer`,
  loads `dartmouth_basic.grammar` via `grammar_tools`, runs the
  `GrammarParser` engine, and returns the root ASTNode with
  `rule_name == "program"`.
- `M.create_parser(source)` — returns a raw `GrammarParser` for
  caller-controlled parsing (useful for trace mode and testing).
- `M.get_grammar()` — returns the cached `ParserGrammar` for
  introspection (e.g., enumerating rule names).
- Grammar caching: the 29-rule BASIC grammar is loaded and compiled
  once per process; subsequent `parse()` calls reuse the cached object.
- Path navigation: 6-level `up()` walk from `init.lua` reaches the
  monorepo `code/` root, then appends `grammars/dartmouth_basic.grammar`.
- Cross-platform path resolution via `io.popen("cd ... && pwd")` so that
  relative paths produced by busted are resolved to absolute paths before
  the `up()` walk.
- Comprehensive test suite (`tests/test_dartmouth_basic_parser.lua`):
  - All 17 BASIC statement types
  - All 6 relational operators in IF statements
  - All expression precedence levels (expr, term, power, unary, primary)
  - Multi-line programs (HELLO WORLD, FOR loop, GOTO loop, GOSUB/RETURN,
    READ/DATA, REM comments)
  - Empty program and bare line number
  - `create_parser` and `get_grammar` API
  - Error cases (missing THEN target, incomplete LET, incomplete FOR)
