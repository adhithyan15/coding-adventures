# Changelog — coding-adventures-javascript-lexer (Lua)

All notable changes to this package are documented here.

## [0.2.0] — 2026-04-05

### Added

- `version` parameter added to `tokenize(source, version)`,
  `create_lexer(source, version)`, and `get_grammar(version)`.
- New `create_lexer(source, version)` function — returns an initialized
  `GrammarLexer` without immediately tokenizing.
- Version routing: when `version` is `"es1"`, `"es3"`, `"es5"`, or
  `"es2015"` through `"es2025"`, the corresponding versioned grammar file is
  loaded from `code/grammars/ecmascript/<version>.tokens`.
- Generic fallback: passing `nil` or `""` loads the unified
  `code/grammars/javascript.tokens` as before (backward compatible).
- Per-version grammar cache: each version is loaded and parsed at most once
  per process; the cache is a table keyed by version string.
- `\v` and `\f` escape normalization applied when loading ECMAScript grammar
  files (Lua's regex engine requires literal control characters instead).
- Validation: unknown version strings raise a descriptive error immediately.
- Extended test suite: new `describe("version-aware tokenization")` block
  covering all recognized ES versions, `create_lexer`, `get_grammar`, cache
  behavior, and error cases for invalid versions.

### Changed

- `M.VERSION` bumped from `"0.1.0"` to `"0.2.0"`.
- `_grammar_cache` changed from a single value to a table keyed by version
  (backward-compatible — no-version calls use key `""`).

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.javascript_lexer`.
- `tokenize(source)` — tokenizes a JavaScript string using the shared
  `javascript.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/javascript.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full token set: NAME, NUMBER, STRING, keyword tokens (LET, CONST, VAR,
  IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM,
  AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED),
  operator tokens (STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS,
  NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS,
  STAR, SLASH, LESS_THAN, GREATER_THAN, BANG), and delimiter tokens
  (LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET, COMMA, COLON,
  SEMICOLON, DOT).
- Comprehensive busted test suite covering keywords, identifiers, numbers,
  strings, operators, punctuation, arrow functions, whitespace handling,
  position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read` (reads grammar
  file at startup).
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.
