# Changelog — tetrad-lexer

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-20

### Added

- `TokenType` enum with all 42 token categories:
  - Literals: `INT` (decimal), `HEX` (0x-prefixed)
  - Identifiers: `IDENT`
  - Keywords: `KW_FN`, `KW_LET`, `KW_IF`, `KW_ELSE`, `KW_WHILE`, `KW_RETURN`, `KW_IN`, `KW_OUT`, `KW_U8`
  - Arithmetic operators: `PLUS`, `MINUS`, `STAR`, `SLASH`, `PERCENT`
  - Bitwise operators: `AMP`, `PIPE`, `CARET`, `TILDE`
  - Shift operators (two-char): `SHL` (`<<`), `SHR` (`>>`)
  - Comparison operators: `EQ_EQ`, `BANG_EQ`, `LT`, `LT_EQ`, `GT`, `GT_EQ`
  - Logical operators: `AMP_AMP` (`&&`), `PIPE_PIPE` (`||`), `BANG`
  - Assignment and annotation: `EQ`, `ARROW` (`->`), `COLON`
  - Delimiters: `LPAREN`, `RPAREN`, `LBRACE`, `RBRACE`, `COMMA`, `SEMI`
  - Sentinel: `EOF`
- `Token` frozen dataclass with `type`, `value`, `line`, `column`, `offset` fields
- `LexError` exception with `.line` and `.column` attributes
- `tokenize(source: str) -> list[Token]` — single-pass maximal-munch scanner:
  - Skips ASCII whitespace (space, tab, CR, LF)
  - Skips C-style line comments (`//` to end of line)
  - Two-char operators checked before one-char prefixes (maximal munch)
  - 1-based line/column tracking; 0-based byte offset tracking
  - Raises `LexError` on the first illegal character or malformed literal
- Full unit test suite (`tests/test_tetrad_lexer.py`): 95%+ line coverage
- `pyproject.toml` with hatchling build backend, ruff + mypy linting, pytest-cov at ≥95% threshold
- `BUILD` / `BUILD_windows` scripts for the repo's custom build tool
