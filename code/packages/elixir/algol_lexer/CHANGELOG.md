# Changelog

## 0.1.0 — 2026-04-06

### Added
- `AlgolLexer.tokenize/1` — tokenize ALGOL 60 source code into a token list
- `AlgolLexer.create_lexer/0` — parse the `algol.tokens` grammar file and return the `TokenGrammar`
- Grammar caching via `persistent_term` for fast repeated calls
- 55 tests covering:
  - Grammar inspection (`create_lexer/0`)
  - All ALGOL 60 keywords: `begin`, `end`, `if`, `then`, `else`, `for`, `do`, `step`, `until`, `while`, `goto`, `switch`, `procedure`, `own`, `array`, `label`, `value`, `integer`, `real`, `boolean`, `string`, `true`, `false`, `not`, `and`, `or`, `impl`, `eqv`, `div`, `mod`
  - Keyword boundary: `beginning` → IDENT, `integer1` → IDENT, `foreach` → IDENT
  - All literal types: `INTEGER_LIT`, `REAL_LIT` (decimal and scientific), `STRING_LIT` (single-quoted)
  - All operators: `:=`, `**`, `^`, `<=`, `>=`, `!=`, `=`, `<`, `>`, `+`, `-`, `*`, `/`
  - All delimiters: `(`, `)`, `[`, `]`, `:`, `;`, `,`
  - Comment skipping: `comment ... ;` consumed silently
  - Whitespace insignificance: `x:=1` identical to `x := 1`
  - Full programs: minimal block, real variable, for loop, if statement, procedure header
  - Position tracking: line and column numbers
  - EOF sentinel always present at end of token stream
  - Error cases: unexpected characters
