# Changelog — CodingAdventures::PythonLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::PythonLexer`.
- `tokenize($source)` — class method that tokenizes a Python string using
  the shared `python.tokens` grammar compiled to Perl `qr//` patterns.
- Grammar loaded from `code/grammars/python.tokens` and cached for the
  process lifetime using package-level variables.
- Path navigation uses `__FILE__` and `File::Basename::dirname` to locate
  the grammar file without hardcoded paths (climbs 5 levels from the module
  file to reach `code/`, then descends into `grammars/`).
- Full token set: NAME, NUMBER, STRING; keyword tokens (IF, ELIF, ELSE,
  WHILE, FOR, DEF, RETURN, CLASS, IMPORT, FROM, AS, TRUE, FALSE, NONE);
  operator tokens (EQUALS_EQUALS, EQUALS, PLUS, MINUS, STAR, SLASH);
  delimiter tokens (LPAREN, RPAREN, COMMA, COLON).
- Line and column tracking throughout tokenization.
- `die` with descriptive message on unexpected input characters.
- EOF sentinel always appended as last token.
- Comprehensive `Test2::V0` test suite covering keywords, identifiers,
  numbers, strings, operators, punctuation, composite expressions,
  whitespace handling, position tracking, and error cases.
- `BUILD` script with transitive dependency installation in leaf-to-root
  order via cpanm.
- `BUILD_windows` stub (Perl testing not supported on Windows).
