# Changelog — coding-adventures-dartmouth-basic-lexer (Lua)

All notable changes to this package are documented here.

## Unreleased

### Fixed

- Eliminated runtime grammar loading: `tokenize` now requires a pre-compiled
  `_grammar` module instead of reading and parsing the `dartmouth_basic.tokens`
  file from `code/grammars/` on every call. The old code walked out of the
  installed package's own directory to a monorepo-relative path that a
  published LuaRocks package does not ship. The `(?:...)` non-capturing-group
  stripping the Lua GrammarLexer needs (see `_grammar.lua`'s BUILTIN_FN
  pattern) is now baked in at compile time instead of applied on every load.

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of `coding_adventures.dartmouth_basic_lexer`.
- `tokenize(source)` — tokenizes a Dartmouth BASIC 1964 string using the
  shared `dartmouth_basic.tokens` grammar and the grammar-driven `GrammarLexer`
  from `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/dartmouth_basic.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative to
  the installed module, avoiding hardcoded absolute paths.
- Post-tokenize processing applied manually (the Lua GrammarLexer has no
  `add_post_tokenize` hook API, unlike the Elixir implementation):
  - `relabel_line_numbers` — promotes the first `NUMBER` token on each source
    line to `LINE_NUM`, implementing positional disambiguation between line
    labels and numeric expressions.
  - `suppress_rem_content` — removes all tokens between a `KEYWORD("REM")` and
    the next `NEWLINE`, implementing Dartmouth BASIC's comment syntax. The
    `NEWLINE` itself is preserved so the parser knows where the REM line ends.
- Supports all Dartmouth BASIC 1964 token types:
  - `LINE_NUM` — line number label at the start of each program line
  - `NUMBER` — numeric literal (integer, decimal, leading-dot, scientific)
  - `STRING` — double-quoted string literal
  - `KEYWORD` — all 20 reserved words: LET, PRINT, INPUT, IF, THEN, GOTO,
    GOSUB, RETURN, FOR, TO, STEP, NEXT, END, STOP, REM, READ, DATA, RESTORE,
    DIM, DEF
  - `BUILTIN_FN` — the 11 built-in functions: SIN, COS, TAN, ATN, EXP, LOG,
    ABS, SQR, INT, RND, SGN
  - `USER_FN` — user-defined functions: FNA–FNZ
  - `NAME` — variable names: single letter (A–Z) or letter+digit (A0–Z9)
  - `LE`, `GE`, `NE` — two-character comparison operators (<=, >=, <>)
  - `PLUS`, `MINUS`, `STAR`, `SLASH`, `CARET`, `EQ`, `LT`, `GT` — arithmetic
    and comparison operators
  - `LPAREN`, `RPAREN`, `COMMA`, `SEMICOLON` — delimiters
  - `NEWLINE` — significant statement terminator (not in skip patterns)
  - `UNKNOWN` — error recovery token for unrecognised characters
  - `EOF` — always the final token
- Case-insensitive tokenization via `@case_insensitive true` in the grammar:
  `print`, `Print`, and `PRINT` all produce `KEYWORD("PRINT")`.
- Multi-character operators matched before single-character variants:
  `<=` before `<`, `>=` before `>`, `<>` before `<`.
- Built-in and user-defined function names matched before `NAME` so that
  `SIN` is not tokenised as `NAME("SI") NAME("N")`.
- Comprehensive busted test suite covering all token types, REM suppression,
  LINE_NUM disambiguation, case insensitivity, multi-character operators,
  NEWLINE handling, position tracking, error recovery, and realistic programs.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar file
  at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order.
