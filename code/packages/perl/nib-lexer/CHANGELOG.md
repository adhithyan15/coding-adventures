# Changelog — CodingAdventures::StarlarkLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::StarlarkLexer`.
- `tokenize($source)` — tokenizes a Starlark string using the shared
  `starlark.tokens` grammar, compiled to Perl `qr//` regexes.
- Grammar is read from `code/grammars/starlark.tokens` once and cached via
  package-level variables.
- Path navigation uses `dirname(__FILE__)` + 5 dirname() levels to reach
  `code/` from `lib/CodingAdventures/`, avoiding hardcoded absolute paths.
- Indentation tracking algorithm implementing `mode: indentation`:
  indentation stack starting at [0], INDENT emitted on level increase,
  DEDENT(s) emitted on level decrease, NEWLINE at each logical line
  boundary, INDENT/DEDENT/NEWLINE suppressed inside (), [], {}.
  Tab characters in leading whitespace cause a SyntaxError.
- Full token set: NAME, INT, FLOAT, STRING; keyword tokens (AND, BREAK,
  CONTINUE, DEF, ELIF, ELSE, FOR, IF, IN, LAMBDA, LOAD, NOT, OR, PASS,
  RETURN, TRUE, FALSE, NONE); three-character operators (DOUBLE_STAR_EQUALS,
  LEFT_SHIFT_EQUALS, RIGHT_SHIFT_EQUALS, FLOOR_DIV_EQUALS); two-character
  operators (DOUBLE_STAR, FLOOR_DIV, LEFT_SHIFT, RIGHT_SHIFT, EQUALS_EQUALS,
  NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS, PLUS_EQUALS, MINUS_EQUALS,
  STAR_EQUALS, SLASH_EQUALS, PERCENT_EQUALS, AMP_EQUALS, PIPE_EQUALS,
  CARET_EQUALS); single-character operators (PLUS, MINUS, STAR, SLASH,
  PERCENT, EQUALS, LESS_THAN, GREATER_THAN, AMP, PIPE, CARET, TILDE);
  delimiter tokens (LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE,
  COMMA, COLON, SEMICOLON, DOT); and indentation tokens (INDENT, DEDENT,
  NEWLINE).
- Comprehensive Test2::V0 test suite: 00-load.t (module loading), 01-basic.t
  (all keywords, identifiers, integers, floats, strings, operators,
  delimiters, indentation mode, comment handling, composite expressions,
  whitespace handling, position tracking, error cases).
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD`, `BUILD_windows`, `Makefile.PL`, and `cpanfile` with transitive
  dependency installation in leaf-to-root order.
