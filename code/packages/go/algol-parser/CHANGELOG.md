# Changelog — algol-parser

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of the ALGOL 60 parser (`parser.go`)
- `CreateAlgolParser(source string)` factory function returning a `*parser.GrammarParser`
- `ParseAlgol(source string)` convenience one-shot parsing function
- Grammar path resolution via `runtime.Caller(0)` (works from any working directory)
- Capability-scoped file I/O via `gen_capabilities.go` (mirrors json-parser pattern)
- Two-stage pipeline: lexing via algol-lexer, parsing via GrammarParser with algol.grammar
- Full test suite (`algol_parser_test.go`) covering:
  - Minimal complete program: `begin integer x; x := 42 end`
  - Simple and chained assignments
  - If/then and if/then/else conditionals with various relational operators
  - For loops (step/until form, simple value form)
  - Arithmetic expressions (+, -, *, /, div, mod, ** exponentiation)
  - Nested begin...end blocks (lexical scoping)
  - Boolean expressions (and, or, not compound conditions)
  - Multiple variable declarations in a single block
  - Real (floating-point) variable declarations and assignments
  - Two-step API via `CreateAlgolParser` then `Parse()`
- `required_capabilities.json` declaring read access to `algol.grammar`
- `README.md` with ALGOL 60 grammar structure, AST example, and usage
