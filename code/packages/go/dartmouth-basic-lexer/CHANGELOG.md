# Changelog — dartmouth-basic-lexer

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of the Dartmouth BASIC 1964 lexer (`lexer.go`)
- `CreateDartmouthBasicLexer(source string)` factory function returning a
  `*lexer.GrammarLexer`, with two post-tokenize hooks registered in order
- `TokenizeDartmouthBasic(source string)` convenience one-shot tokenization
  function
- Grammar path resolution via `runtime.Caller(0)` so tests and the build tool
  can run from any working directory
- Capability-scoped file I/O via `gen_capabilities.go` (mirrors algol-lexer and
  json-lexer pattern); only `dartmouth_basic.tokens` may be read
- `relabelLineNumbers` post-tokenize hook that reclassifies the first NUMBER
  token on each source line as LINE_NUM, enabling the parser to distinguish line
  labels from numeric literals in expressions
- `suppressRemContent` post-tokenize hook that discards all tokens between a
  KEYWORD("REM") and the next NEWLINE (the NEWLINE itself is preserved, since it
  is the statement terminator)
- Dartmouth BASIC grammar file (`code/grammars/dartmouth_basic.tokens`) with:
  - `case_sensitive: false` directive (source lowercased before matching)
  - `# @case_insensitive true` comment (keywords normalized to uppercase)
  - Multi-character operators: LE (`<=`), GE (`>=`), NE (`<>`)
  - NUMBER regex covering integer, decimal, leading-dot, and scientific notation
  - STRING_BODY aliased to STRING (surrounding quotes stripped by lexer)
  - BUILTIN_FN covering all 11 original built-ins: SIN, COS, TAN, ATN, EXP,
    LOG, ABS, SQR, INT, RND, SGN
  - USER_FN pattern for user-defined functions FNA through FNZ
  - NAME regex `/[a-z][a-z0-9]*/` (multi-character for keyword promotion)
  - All 20 Dartmouth BASIC 1964 keywords: LET, PRINT, INPUT, IF, THEN, GOTO,
    GOSUB, RETURN, FOR, TO, STEP, NEXT, END, STOP, REM, READ, DATA, RESTORE,
    DIM, DEF
  - Single-character operators: PLUS, MINUS, STAR, SLASH, CARET, EQ, LT, GT,
    LPAREN, RPAREN, COMMA, SEMICOLON
  - NEWLINE kept in token stream (statement terminator)
  - WHITESPACE in skip section (spaces and tabs consumed silently)
- Full test suite (`dartmouth_basic_lexer_test.go`) with 96%+ coverage covering:
  - All 20 keywords (LET, PRINT, INPUT, IF/THEN, GOTO, GOSUB/RETURN, FOR/TO/STEP,
    NEXT, END, STOP, REM, READ/DATA/RESTORE, DIM, DEF, INPUT)
  - All 11 built-in functions (SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT,
    RND, SGN)
  - User-defined functions FNA, FNB, FNZ
  - Case insensitivity: "print", "PRINT", "Print" all → KEYWORD("PRINT")
  - Multi-character operators: <=, >=, <>
  - All single-character operators
  - Number formats: integer, decimal, leading-dot, scientific notation (1.5e3)
  - String literals with original case preserved, quotes stripped
  - REM suppression: comment text absent, NEWLINE preserved
  - REM followed by more code on the next line
  - LINE_NUM vs NUMBER disambiguation across multi-line programs
  - PRINT separators: semicolon and comma
  - Windows line endings (\r\n)
  - GOSUB/RETURN subroutine pattern
  - Line and column tracking for error reporting
  - Complex multi-statement program
  - Keyword non-splitting: "PRINT" is one KEYWORD, not BUILTIN_FN("rint") etc.
  - Capability cage error paths: violation error, Fail, AddProperty,
    PanicCaught, PanicOnUnexpected, UnexpectedFailure, ExpectedFailureNoErr
- `required_capabilities.json` declaring read access to `dartmouth_basic.tokens`
- `BUILD` and `BUILD_windows` files for the repo's Go build tool
- `README.md` with Dartmouth BASIC history, token table, and usage examples
