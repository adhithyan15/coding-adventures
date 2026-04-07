# Changelog — @coding-adventures/dartmouth-basic-lexer

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-06

### Added

- Initial release of the Dartmouth BASIC 1964 lexer for TypeScript.
- `tokenizeDartmouthBasic(source: string): Token[]` — primary entry point.
  Tokenizes a complete BASIC source string and returns the full token list
  including EOF. Applies both post-tokenize hooks automatically.
- `createDartmouthBasicLexer(source: string): GrammarLexer` — factory function
  that returns a pre-configured `GrammarLexer` for callers who need to attach
  additional post-tokenize hooks.
- Grammar-driven tokenization using `dartmouth_basic.tokens` via the
  `@coding-adventures/grammar-tools` and `@coding-adventures/lexer` packages.
- **Post-tokenize hook: `relabelLineNumbers`** — walks the token list and
  relabels the first `NUMBER`/`LINE_NUM` token on each source line as
  `LINE_NUM`, reflecting its role as a line label. All other integer tokens
  become `NUMBER`. This hook correctly handles multi-line programs and
  preserves GOTO/GOSUB/IF...THEN targets as `NUMBER` tokens.
- **Post-tokenize hook: `suppressRemContent`** — walks the token list and
  drops all tokens between a `KEYWORD("REM")` and the next `NEWLINE`. The
  `KEYWORD("REM")` token itself and the terminating `NEWLINE` are preserved.
- Support for all 20 Dartmouth BASIC 1964 reserved words: `LET`, `PRINT`,
  `INPUT`, `IF`, `THEN`, `GOTO`, `GOSUB`, `RETURN`, `FOR`, `TO`, `STEP`,
  `NEXT`, `END`, `STOP`, `REM`, `READ`, `DATA`, `RESTORE`, `DIM`, `DEF`.
- Support for all 11 built-in mathematical functions as `BUILTIN_FN` tokens:
  `SIN`, `COS`, `TAN`, `ATN`, `EXP`, `LOG`, `ABS`, `SQR`, `INT`, `RND`,
  `SGN`.
- Support for user-defined functions (`USER_FN`) of the form `FNA`–`FNZ`.
- Support for variable names (`NAME`) matching `/[A-Z][0-9]?/`: one letter
  optionally followed by one digit (A–Z, A0–Z9: 286 names total).
- Case-insensitive tokenization via the `@case_insensitive true` directive
  in `dartmouth_basic.tokens`. Input is uppercased before matching.
- Multi-character operator disambiguation: `<=` (LE), `>=` (GE), `<>` (NE)
  are matched before their component single-character forms.
- `NEWLINE` tokens are emitted (not skipped) because they are syntactically
  significant statement terminators in BASIC.
- Error recovery: unrecognised characters produce `UNKNOWN` tokens rather
  than throwing an exception.
- Comprehensive vitest test suite with 60+ test cases covering all token
  types, both post-tokenize hooks, case insensitivity, operator
  disambiguation, error recovery, and position tracking.
- `BUILD` and `BUILD_windows` scripts for the monorepo build system.
- `required_capabilities.json` declaring the filesystem read capability
  for the grammar file.
