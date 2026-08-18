# Changelog — coding-adventures-python-parser

## [Unreleased]

### Fixed

- Eliminated runtime grammar loading. Previously, `get_grammar()` read
  `code/grammars/python/python.grammar` off disk at runtime using a path
  that walked outside this package's own directory into the monorepo
  (`debug.getinfo`-based directory walk-up). That works inside a checkout
  of the monorepo, but a published LuaRocks package does not include
  `code/grammars/` — installing this rock and calling `parse` would raise
  a file-not-found error.
- The single `python.grammar` (this package is not versioned — `parse()`
  and `create_parser()` take no version argument, unlike `python_lexer`)
  is now compiled ahead of time (via `grammar-tools compile-grammar`) into
  a `_grammar_default.lua` sibling module that embeds the parsed
  `ParserGrammar` as native Lua data. `init.lua` now `require`s this
  precompiled module and calls its `parser_grammar()` constructor (cached)
  instead of reading and parsing `python.grammar` from disk.
- The rockspec's `build.modules` table now lists the `_grammar_default.lua`
  submodule explicitly so `luarocks install` actually packages it.
- Public API (`parse`, `create_parser`, `get_grammar`) is unchanged.

## [0.1.0] — 2026-03-29

### Added
- Initial implementation of the grammar-driven Python parser.
- `parse(source)` — tokenizes with `python_lexer`, loads `python.grammar`,
  runs `GrammarParser`, and returns the root `ASTNode`.
- `create_parser(source)` — returns an initialized `GrammarParser` without
  immediately parsing, for trace-mode or custom parsing workflows.
- `get_grammar()` — returns the cached `ParserGrammar` for inspection.
- Grammar-file caching: `python.grammar` is loaded once and reused.
- Supports assignments (`x = 5`), arithmetic with correct operator precedence
  (`+`/`-` at expression level, `*`/`/` at term level), parenthesized groups,
  and expression statements.
- Full busted test suite in `tests/test_python_parser.lua` covering:
  module API, root node structure, assignments, expression statements,
  operator precedence, multiple statements, grammar inspection, and
  error handling.
- `required_capabilities.json` declaring `filesystem:read` capability.
- `BUILD` and `BUILD_windows` for the monorepo build system.
- `README.md` with usage examples, grammar listing, and stack diagram.
