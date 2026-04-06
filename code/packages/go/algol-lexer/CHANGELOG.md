# Changelog — algol-lexer

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of the ALGOL 60 lexer (`lexer.go`)
- `CreateAlgolLexer(source string)` factory function returning a `*lexer.GrammarLexer`
- `TokenizeAlgol(source string)` convenience one-shot tokenization function
- Grammar path resolution via `runtime.Caller(0)` (works from any working directory)
- Capability-scoped file I/O via `gen_capabilities.go` (mirrors json-lexer pattern)
- Full test suite (`algol_lexer_test.go`) covering:
  - All 29 ALGOL 60 keywords (BEGIN, END, IF, THEN, ELSE, FOR, DO, STEP, UNTIL,
    WHILE, GOTO, INTEGER, REAL, BOOLEAN, STRING, PROCEDURE, ARRAY, SWITCH, OWN,
    LABEL, VALUE, TRUE, FALSE, NOT, AND, OR, IMPL, EQV, DIV, MOD)
  - `:=` (ASSIGN) vs `=` (EQ) disambiguation
  - All operators including `**` (POWER) and `^` (CARET) for exponentiation
  - Integer literals (0, 42, 1000)
  - Real literals (3.14, 1.5E3, 1.5E-3, 100E2)
  - String literals (`'single-quoted'`)
  - Comment skipping (`comment text;`)
  - Keyword boundary: `beginning` is IDENT, not BEGIN
  - Full expression: `x := 1 + 2 * 3`
  - Whitespace insignificance: `x:=1` equals `x := 1`
  - EOF token always present
  - Line and column tracking
  - Minimal complete program: `begin integer x; x := 42 end`
- `required_capabilities.json` declaring read access to `algol.tokens`
- `README.md` with ALGOL 60 history, token table, and usage examples
