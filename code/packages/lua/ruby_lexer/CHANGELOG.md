# Changelog — coding-adventures-ruby-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.ruby_lexer`.
- `tokenize(source)` — tokenizes a Ruby string using the shared
  `ruby.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/ruby.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set: NAME, NUMBER, STRING, keyword tokens (DEF, END, CLASS,
  MODULE, IF, ELSIF, ELSE, UNLESS, WHILE, UNTIL, FOR, DO, RETURN, BEGIN,
  RESCUE, ENSURE, REQUIRE, PUTS, YIELD, THEN, TRUE, FALSE, NIL, AND, OR,
  NOT), multi-char operator tokens (EQUALS_EQUALS, DOT_DOT, HASH_ROCKET,
  NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS), single-char operator tokens
  (EQUALS, PLUS, MINUS, STAR, SLASH, LESS_THAN, GREATER_THAN), and
  delimiter tokens (LPAREN, RPAREN, COMMA, COLON).
- Comprehensive busted test suite covering all keywords, identifiers,
  numbers, strings, operators (including Ruby-specific `..` and `=>`),
  punctuation, composite expressions, whitespace handling, position
  tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
