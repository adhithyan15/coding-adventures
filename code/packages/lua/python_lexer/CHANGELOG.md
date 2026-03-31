# Changelog — coding-adventures-python-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.python_lexer`.
- `tokenize(source)` — tokenizes a Python string using the shared
  `python.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/python.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set: NAME, NUMBER, STRING, keyword tokens (IF, ELIF, ELSE,
  WHILE, FOR, DEF, RETURN, CLASS, IMPORT, FROM, AS, TRUE, FALSE, NONE),
  operator tokens (EQUALS_EQUALS, EQUALS, PLUS, MINUS, STAR, SLASH), and
  delimiter tokens (LPAREN, RPAREN, COMMA, COLON).
- Comprehensive busted test suite covering keywords, identifiers, numbers,
  strings, operators, punctuation, composite expressions, whitespace
  handling, position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
