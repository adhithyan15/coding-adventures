# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial release of the Dartmouth BASIC lexer crate.
- `create_dartmouth_basic_lexer()` factory function returning a configured
  `GrammarLexer` with both post-tokenize hooks registered.
- `tokenize_dartmouth_basic()` convenience function returning `Vec<Token>`.
- Loads the `dartmouth_basic.tokens` grammar file at runtime from the shared
  `code/grammars/` directory.
- Supports all 1964 Dartmouth BASIC token types:
  - `LINE_NUM` — line label at the start of each BASIC statement line
  - `NUMBER` — numeric literal (integer, decimal, scientific notation)
  - `STRING` — double-quoted string literal (no escape sequences)
  - `KEYWORD` — all 20 reserved words: LET, PRINT, INPUT, IF, THEN, GOTO,
    GOSUB, RETURN, FOR, TO, STEP, NEXT, END, STOP, REM, READ, DATA,
    RESTORE, DIM, DEF
  - `BUILTIN_FN` — all 11 built-in functions: SIN, COS, TAN, ATN, EXP, LOG,
    ABS, SQR, INT, RND, SGN
  - `USER_FN` — user-defined functions FNA through FNZ
  - `NAME` — variable names (one letter, or letter+digit)
  - Operators: PLUS, MINUS, STAR, SLASH, CARET, EQ, LT, GT, LE, GE, NE
  - Punctuation: LPAREN, RPAREN, COMMA, SEMICOLON
  - Structure: NEWLINE (kept — significant statement terminator)
  - Error recovery: UNKNOWN (for unrecognized characters)
- Post-tokenize hook 1: `relabel_line_numbers` — fixes integer tokens that
  are not at line-number position from LINE_NUM back to NUMBER.
- Post-tokenize hook 2: `suppress_rem_content` — removes all tokens between
  a REM keyword and the end of its line, implementing BASIC comment syntax.
- Case-insensitive tokenization via `@case_insensitive true` in the grammar —
  `print` and `PRINT` produce identical KEYWORD("PRINT") tokens.
- 35 unit tests covering all token types, both post-tokenize hooks, case
  insensitivity, all number formats, all operators, error recovery, and
  the factory function.
