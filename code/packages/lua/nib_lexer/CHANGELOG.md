# Changelog — coding-adventures-starlark-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.starlark_lexer`.
- `tokenize(source)` — tokenizes a Starlark string using the shared
  `starlark.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/starlark.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
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
  NEWLINE) emitted automatically by `mode: indentation`.
- Comprehensive busted test suite covering keywords, identifiers, integers,
  floats, strings, three/two/one-character operators, delimiters, indentation
  mode, comment handling, composite expressions, whitespace handling, position
  tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
