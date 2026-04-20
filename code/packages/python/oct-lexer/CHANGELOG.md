# Changelog

## [0.1.0] - 2026-04-20

### Added

- Initial implementation of the Oct lexer — a thin wrapper around the generic
  `GrammarLexer` that loads `code/grammars/oct.tokens`.
- `create_oct_lexer(source)` — creates a `GrammarLexer` configured for Oct,
  returning a lexer instance whose `.tokenize()` method produces the raw token
  list.
- `tokenize_oct(source)` — the main entry point; tokenizes Oct source text and
  applies a post-processing pass that promotes `TokenType.KEYWORD` tokens to
  use the keyword string as their type (e.g. `Token("fn", "fn")`), matching the
  convention expected by `oct-parser` and downstream consumers.
- `OCT_TOKENS_PATH` — the path constant pointing at `code/grammars/oct.tokens`,
  resolved relative to the module file so the package works correctly regardless
  of the current working directory.
- Token definitions in `code/grammars/oct.tokens` (committed alongside this
  package):
  - Multi-character operators: `EQ_EQ` (`==`), `NEQ` (`!=`), `LEQ` (`<=`),
    `GEQ` (`>=`), `LAND` (`&&`), `LOR` (`||`), `ARROW` (`->`)
  - Single-character arithmetic: `PLUS` (`+`), `MINUS` (`-`)
  - Single-character bitwise: `AMP` (`&`), `PIPE` (`|`), `CARET` (`^`),
    `TILDE` (`~`), `BANG` (`!`), `LT` (`<`), `GT` (`>`)
  - Assignment: `EQ` (`=`)
  - Delimiters: `LBRACE`, `RBRACE`, `LPAREN`, `RPAREN`, `COLON`, `SEMICOLON`,
    `COMMA`
  - Literals: `BIN_LIT` (`0b…`), `HEX_LIT` (`0x…`), `INT_LIT` (decimal),
    `NAME` (identifiers including type names `u8` and `bool`)
  - Keywords: `fn`, `let`, `static`, `if`, `else`, `while`, `loop`, `break`,
    `return`, `true`, `false`, `in`, `out`, `adc`, `sbb`, `rlc`, `rrc`,
    `ral`, `rar`, `carry`, `parity`
  - Skip patterns: `WHITESPACE`, `LINE_COMMENT` (`// …`)
- Comprehensive test suite (`tests/test_oct_lexer.py`):
  - `TestTokenizeOct` — 60 tests covering all token kinds, keyword promotion,
    intrinsic keyword tokens, binary/hex/decimal literals, operator precedence
    ordering, whitespace/comment skipping, multi-token expressions, and all
    five complete Oct program examples from the OCT00 spec.
