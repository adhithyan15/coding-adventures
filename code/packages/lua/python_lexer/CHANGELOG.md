# Changelog — coding-adventures-python-lexer (Lua)

All notable changes to this package are documented here.

## [Unreleased]

### Fixed

- Eliminated runtime grammar loading. Previously, `get_grammar(version)`
  read `code/grammars/python/python<version>.tokens` off disk at runtime
  using a path that walked outside this package's own directory into the
  monorepo (`debug.getinfo`-based directory walk-up). That works inside a
  checkout of the monorepo, but a published LuaRocks package does not
  include `code/grammars/` — installing this rock and calling `tokenize`
  would raise a file-not-found error.
- Each of the 6 supported Python versions' `.tokens` grammar is now
  compiled ahead of time (via `grammar-tools compile-tokens`) into a
  `_grammar_<version>.lua` sibling module. Since Python version strings
  contain dots (e.g. `"3.12"`), the filenames/module names substitute
  underscores for dots (e.g. `_grammar_3_12.lua`) while the public-facing
  version string stays exactly `"3.12"`. `init.lua` now `require`s the
  appropriate precompiled module and calls its `token_grammar()`
  constructor (cached per version) instead of reading and parsing a
  `.tokens` file from disk.
- Unknown version strings now raise an explicit
  `"python_lexer: unknown Python version '<v>'. Valid values are: 2.7, 3.0,
  3.6, 3.8, 3.10, 3.12."` error instead of a raw
  `io.open`-failure message, since there is no longer a file path to
  report — this is not covered by the existing test suite (no test
  asserted on the old file-not-found message text).
- The rockspec's `build.modules` table now lists every `_grammar_*.lua`
  submodule explicitly so `luarocks install` actually packages them.
- Public API (`tokenize`, `get_grammar`, `M.DEFAULT_VERSION`,
  `M.SUPPORTED_VERSIONS`) is otherwise unchanged.

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
