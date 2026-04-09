# Changelog — dartmouth-basic-lexer

All notable changes to this package are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Version numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-06

### Added

- **Initial release** of `dartmouth_basic_lexer`, tokenizing the 1964
  Dartmouth BASIC language designed by John Kemeny and Thomas Kurtz.

- **`tokenize_dartmouth_basic(source)`** — main public function. Tokenizes
  a string of BASIC source text and returns a list of `Token` objects.
  Applies both post-tokenize hooks automatically. Always appends an EOF
  token at the end.

- **`create_dartmouth_basic_lexer(source)`** — factory function returning a
  raw `GrammarLexer` without the post-tokenize hooks attached. Use this
  when you need direct access to the lexer to add custom transformations.

- **`relabel_line_numbers(tokens)`** — post-tokenize hook that walks the
  token list and relabels the first `NUMBER` on each source line as
  `LINE_NUM`. This disambiguates line labels (``10 LET X = 5``) from
  numeric literals in expressions (``LET X = 42``).

- **`suppress_rem_content(tokens)`** — post-tokenize hook that removes
  all tokens between a `KEYWORD("REM")` and the next `NEWLINE`. The
  NEWLINE is preserved as the statement terminator. This implements
  BASIC's comment syntax.

- **Grammar file** (`code/grammars/dartmouth_basic.tokens`): shared
  grammar supporting all 20 keywords, 11 built-in functions, user-defined
  functions (FNA–FNZ), scientific notation numerics, double-quoted strings,
  all comparison and arithmetic operators, and error recovery via `UNKNOWN`.

- **Token types produced**:
  - `LINE_NUM` — integer at start of a source line (line label)
  - `NUMBER`   — numeric literal in an expression
  - `STRING`   — double-quoted string literal (includes quotes in value)
  - `KEYWORD`  — one of 20 reserved words, always uppercase
  - `BUILTIN_FN` — one of 11 built-in math functions
  - `USER_FN`  — user-defined function FNA through FNZ
  - `NAME`     — variable name: one letter + optional digit
  - `PLUS`, `MINUS`, `STAR`, `SLASH`, `CARET` — arithmetic operators
  - `EQ`, `LT`, `GT`, `LE`, `GE`, `NE` — comparison operators
  - `LPAREN`, `RPAREN`, `COMMA`, `SEMICOLON` — punctuation
  - `NEWLINE`  — statement terminator (significant in BASIC)
  - `EOF`      — always the last token
  - `UNKNOWN`  — unrecognized character (error recovery)

- **Comprehensive test suite** (`tests/test_dartmouth_basic_lexer.py`):
  20 test classes covering factory function, basic tokenization, case
  insensitivity, multi-char operators, all number formats, string literals,
  REM suppression, multi-line programs, LINE_NUM vs NUMBER disambiguation,
  all 11 built-in functions, user-defined functions, PRINT separators,
  variable names, error recovery, FOR/TO/STEP/NEXT keywords, all 20
  keywords, arithmetic operators, token positions, and edge cases.
  Coverage target: ≥ 95%.

- **Package files**: `pyproject.toml`, `BUILD`, `BUILD_windows`,
  `required_capabilities.json`, `README.md`, `CHANGELOG.md`.

### Implementation Notes

- The post-tokenize hook approach was chosen over an `on_token` callback
  because the completed token list is easier to reason about for
  positional disambiguation (LINE_NUM vs NUMBER).

- The grammar uses `@case_insensitive true` which uppercases the entire
  source before matching. This is historically accurate: the 1964
  Dartmouth teletypes had no lowercase keys.

- `BUILTIN_FN` and `USER_FN` appear before the `NAME` rule in the grammar
  to ensure that `SIN` is tokenized as a built-in, not as a series of
  single-letter NAME tokens.

- `LE`, `GE`, `NE` appear before `LT`, `GT`, `EQ` in the grammar so that
  `<=` matches as one LE token rather than LT + EQ.
