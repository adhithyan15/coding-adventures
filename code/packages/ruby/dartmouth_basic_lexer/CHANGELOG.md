# Changelog

All notable changes to `coding_adventures_dartmouth_basic_lexer` will be documented in this file.

## [0.1.0] - 2026-04-06

### Added
- Initial release
- `CodingAdventures::DartmouthBasicLexer.tokenize(source)` method that tokenizes Dartmouth BASIC 1964 source text
- Loads `dartmouth_basic.tokens` grammar file from the shared `code/grammars/` directory and delegates to `GrammarLexer`
- `@case_insensitive true` grammar directive: all input normalised to uppercase before matching; keywords emitted in uppercase
- Support for all 20 reserved keywords: `LET`, `PRINT`, `INPUT`, `IF`, `THEN`, `GOTO`, `GOSUB`, `RETURN`, `FOR`, `TO`, `STEP`, `NEXT`, `END`, `STOP`, `REM`, `READ`, `DATA`, `RESTORE`, `DIM`, `DEF`
- Support for all 11 built-in functions as `BUILTIN_FN` tokens: `SIN`, `COS`, `TAN`, `ATN`, `EXP`, `LOG`, `ABS`, `SQR`, `INT`, `RND`, `SGN`
- Support for user-defined functions as `USER_FN` tokens: `FNA`..`FNZ`
- Support for variable names as `NAME` tokens: one letter plus optional digit (`A`..`Z`, `A0`..`Z9`)
- Support for numeric literals as `NUMBER` tokens: integers, decimals, leading-dot numbers, and scientific notation (`1.5E3`, `1.5E-3`)
- Support for string literals as `STRING` tokens (surrounding double quotes stripped by the lexer engine)
- Multi-character comparison operators: `LE` (`<=`), `GE` (`>=`), `NE` (`<>`) with priority over single-character prefixes
- Single-character operators: `PLUS`, `MINUS`, `STAR`, `SLASH`, `CARET`, `EQ`, `LT`, `GT`
- Delimiters: `LPAREN`, `RPAREN`, `COMMA`, `SEMICOLON`
- `NEWLINE` tokens preserved in stream (significant in BASIC — they terminate statements)
- `UNKNOWN` tokens for unrecognised characters (error recovery — lexer continues past bad input)
- **Post-tokenize hook 1 — LINE_NUM relabelling:** The first `NUMBER` token on each physical line is relabelled `LINE_NUM`, disambiguating line labels from numeric literals in expressions
- **Post-tokenize hook 2 — REM suppression:** All tokens between a `KEYWORD("REM")` and the next `NEWLINE` are dropped (comment text never appears in the output; the `REM` token and `NEWLINE` are preserved)
- Comprehensive test suite covering all token types, all keywords, all built-in functions, LINE_NUM disambiguation, REM suppression, case insensitivity, multi-line programs, error recovery, line/column tracking, and whitespace handling
- SimpleCov coverage reporting with minimum 80% coverage gate
