# Changelog

## 0.1.0 — 2026-04-06

### Added

- `DartmouthBasicLexer.tokenize/1` — tokenize Dartmouth BASIC 1964 source code
  into a flat list of typed tokens, terminated by `EOF`.
- `DartmouthBasicLexer.create_lexer/0` — parse the `dartmouth_basic.tokens`
  grammar file and return the `TokenGrammar` struct for inspection.
- Grammar caching via `:persistent_term` — the grammar file is read from disk
  once per BEAM node lifetime; all subsequent calls to `tokenize/1` use the
  cached copy with zero disk I/O.
- **Post-tokenize hook 1: `relabel_line_numbers/1`** — walks the token list
  with a two-state machine (`:at_line_start` / `:in_line`) and relabels the
  first `NUMBER` token on each physical line as `LINE_NUM`. This disambiguates
  the leading line label (`10 LET X = 5`) from numeric literals in expressions
  (`LET X = 42`) and GOTO targets (`GOTO 10`).
- **Post-tokenize hook 2: `suppress_rem_content/1`** — when a `KEYWORD("REM")`
  token is seen, suppresses all tokens until the next `NEWLINE`. The `REM`
  keyword and the `NEWLINE` are preserved; all comment text is dropped. This
  implements the 1964 Dartmouth BASIC comment mechanism without requiring a
  special grammar mode.
- Grammar file `code/grammars/dartmouth_basic.tokens` — shared token grammar
  for the Dartmouth BASIC 1964 dialect, covering:
  - Multi-character comparison operators (`LE`, `GE`, `NE`) with priority over
    single-char operators to prevent `<=` splitting into `LT` + `EQ`
  - `LINE_NUM` and `NUMBER` (same regex, disambiguated by the hook)
  - `NUMBER` supporting integers, decimals, leading-dot decimals, and
    scientific notation with optional sign on the exponent
  - `STRING` (double-quoted, no escape sequences — authentic to 1964 spec)
  - `BUILTIN_FN` — all 11 built-in functions: `SIN`, `COS`, `TAN`, `ATN`,
    `EXP`, `LOG`, `ABS`, `SQR`, `INT`, `RND`, `SGN`
  - `USER_FN` — user-defined functions `FNA`–`FNZ`
  - `NAME` — variable names: one uppercase letter plus optional digit
  - 20 `keywords:`: `LET`, `PRINT`, `INPUT`, `IF`, `THEN`, `GOTO`, `GOSUB`,
    `RETURN`, `FOR`, `TO`, `STEP`, `NEXT`, `END`, `STOP`, `REM`, `READ`,
    `DATA`, `RESTORE`, `DIM`, `DEF`
  - Arithmetic and comparison operators: `PLUS`, `MINUS`, `STAR`, `SLASH`,
    `CARET`, `EQ`, `LT`, `GT`
  - Delimiters: `LPAREN`, `RPAREN`, `COMMA`, `SEMICOLON`
  - `NEWLINE` (significant — kept in token stream as statement terminator)
  - `skip:` section for horizontal whitespace (`WHITESPACE`)
  - `errors:` section with `UNKNOWN` for error recovery
  - `@case_insensitive true` directive — whole source uppercased before
    matching, matching the behaviour of 1964 uppercase-only teletypes
- Test suite with 100+ test cases covering:
  - Grammar inspection via `create_lexer/0`
  - Canonical LET statement tokenisation and EOF sentinel
  - LINE_NUM disambiguation: line label vs. GOTO target vs. expression literal
  - REM suppression: comment text dropped, NEWLINE preserved, post-REM lines
    tokenise normally
  - Case insensitivity for all 20 keywords, variable names, and built-in functions
  - All 20 keywords individually and collectively
  - NUMBER literal formats: integer, decimal, leading-dot, scientific notation
    with positive and negative exponents
  - STRING literals including quotes, spaces, empty strings
  - Multi-character operators: `LE`, `GE`, `NE` take priority over single-char
  - All single-character operators and punctuation
  - NEWLINE handling: Unix `\n` and Windows `\r\n`
  - All 11 BUILTIN_FN tokens
  - USER_FN tokens (FNA, FNB, FNZ)
  - Variable name forms (single letter, letter+digit)
  - Full multi-line programs: LET/PRINT/END, FOR/NEXT, IF/THEN, GOSUB/RETURN,
    REM comments, DEF/USER_FN, trigonometric computation
  - PRINT separators: COMMA and SEMICOLON
  - Position tracking: line and column numbers
  - Error recovery: UNKNOWN tokens for invalid characters, tokenisation continues
  - Whitespace: spaces and tabs skipped, NEWLINE significant
  - Edge cases: DATA, DIM, INPUT, negative numbers, nested parentheses, no
    trailing newline, RESTORE, STOP
