# Changelog

All notable changes to `coding_adventures_algol_lexer` will be documented in this file.

## [0.1.0] - 2026-04-06

### Added
- Initial release
- `CodingAdventures::AlgolLexer.tokenize(source)` method that tokenizes ALGOL 60 source text
- Loads `algol.tokens` grammar file and delegates to `GrammarLexer`
- Supports value tokens: `REAL_LIT`, `INTEGER_LIT`, `STRING_LIT`, `IDENT`
- Supports 30 keywords reclassified from `IDENT`: `begin`, `end`, `if`, `then`, `else`, `for`, `do`, `step`, `until`, `while`, `goto`, `switch`, `procedure`, `own`, `array`, `label`, `value`, `integer`, `real`, `boolean`, `string`, `true`, `false`, `not`, `and`, `or`, `impl`, `eqv`, `div`, `mod`
- Supports multi-character operators: `ASSIGN` (`:=`), `POWER` (`**`), `LEQ` (`<=`), `GEQ` (`>=`), `NEQ` (`!=`)
- Supports single-character operators: `PLUS`, `MINUS`, `STAR`, `SLASH`, `CARET`, `EQ`, `LT`, `GT`
- Supports delimiters: `LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `SEMICOLON`, `COMMA`, `COLON`
- Whitespace (space, tab, CR, LF) silently skipped — no NEWLINE/INDENT/DEDENT tokens
- ALGOL 60 comments (`comment <text>;`) silently skipped
- Correct operator priority: multi-character operators always precede their single-character prefixes
- Keyword boundary enforcement: `beginning` lexes as IDENT, not `BEGIN` + IDENT(`ning`)
- Full test suite with SimpleCov coverage >= 80%
