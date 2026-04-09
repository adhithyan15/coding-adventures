# Changelog — CodingAdventures::DartmouthBasicLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-04-06

### Added

- Initial implementation of `CodingAdventures::DartmouthBasicLexer`.
- `tokenize($source)` — tokenizes a Dartmouth BASIC 1964 string using rules
  compiled from the shared `dartmouth_basic.tokens` grammar file.
- Grammar is read from `code/grammars/dartmouth_basic.tokens` once and cached
  in package-level variables (`$_grammar`, `$_rules`, `$_skip_rules`, `$_keywords`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root
  (same depth as algol-lexer and json-lexer).
- Case normalisation: the entire source is uppercased with `uc()` before
  tokenizing, reflecting the `@case_insensitive true` grammar directive.
  This means `print`, `Print`, and `PRINT` all produce `KEYWORD("PRINT")`.
- Post-tokenize transformation 1 — `_relabel_line_numbers`: relabels the first
  NUMBER token on each source line as LINE_NUM, disambiguating line labels from
  numeric expressions. (`10 LET X = 5` → LINE_NUM(10); `GOTO 10` → NUMBER(10))
- Post-tokenize transformation 2 — `_suppress_rem_content`: drops all tokens
  between a `KEYWORD("REM")` and the following NEWLINE, implementing BASIC's
  line comment syntax.
- Keyword reclassification: any NAME whose value appears in the keywords: table
  (`LET`, `PRINT`, `INPUT`, `IF`, `THEN`, `GOTO`, `GOSUB`, `RETURN`, `FOR`,
  `TO`, `STEP`, `NEXT`, `END`, `STOP`, `REM`, `READ`, `DATA`, `RESTORE`,
  `DIM`, `DEF`) is promoted to type `KEYWORD`.
- `BUILTIN_FN` matched before NAME so `SIN`, `COS`, etc. are not split.
- `USER_FN` pattern `FN[A-Z]` matched before NAME so `FNA`..`FNZ` are correct.
- Multi-character operator priority: `LE`, `GE`, `NE` match before their
  single-character components (`<`, `>`, `=`).
- NEWLINE tokens kept in the stream (statement terminators, not skipped).
- Horizontal whitespace silently consumed via skip patterns.
- `UNKNOWN = /./` catch-all produces UNKNOWN tokens for unrecognized
  characters instead of dying, enabling error recovery.
- Line and column tracking for all tokens.
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering:
  - Empty and whitespace-only input
  - LINE_NUM disambiguation (at line start vs inside statement)
  - All 20 BASIC keywords
  - Case insensitivity (lowercase and mixed-case input)
  - All 11 built-in functions (SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR, INT, RND, SGN)
  - User-defined functions (FNA, FNB, FNZ)
  - Variable names (single letter, letter+digit)
  - All numeric literal formats (integer, decimal, leading-dot, scientific, neg-exponent)
  - String literals (basic, empty, with punctuation)
  - Multi-character operators (LE, GE, NE priority over LT, GT, EQ)
  - All single-character operators (PLUS, MINUS, STAR, SLASH, CARET, EQ, LT, GT)
  - Delimiters (LPAREN, RPAREN, COMMA, SEMICOLON)
  - NEWLINE in stream and Windows CRLF
  - REM suppression (with comment text, empty REM, multi-line programs)
  - Whitespace handling (extra spaces, no spaces)
  - Complete statement types: FOR/NEXT, GOSUB/RETURN, DIM, READ/DATA/RESTORE,
    INPUT, STOP, arithmetic, comparisons, parenthesized expressions
  - EOF sentinel always last
  - UNKNOWN error recovery tokens
  - Line/column position tracking
- `BUILD` and `BUILD_windows` scripts.
- `Makefile.PL` and `cpanfile`.
- `required_capabilities.json`.
