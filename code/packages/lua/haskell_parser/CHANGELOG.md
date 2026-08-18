# Changelog — coding-adventures-haskell-parser

## [Unreleased]

### Fixed

- Eliminated runtime grammar loading. Previously, `get_grammar(version)`
  read `code/grammars/haskell/haskell<version>.grammar` off disk at
  runtime using a path that walked outside this package's own directory
  into the monorepo (`debug.getinfo`-based directory walk-up). That works
  inside a checkout of the monorepo, but a published LuaRocks package
  does not include `code/grammars/` — installing this rock and calling
  `parse` would raise a file-not-found error.
- Each of the 7 supported Haskell versions' `.grammar` file
  (`1.0`, `1.1`, `1.2`, `1.3`, `1.4`, `98`, `2010`) is now compiled ahead
  of time (via `grammar-tools compile-grammar`) into a
  `_grammar_<version>.lua` sibling module (e.g. `_grammar_2010.lua`) that
  embeds the parsed `ParserGrammar` as native Lua data. `init.lua` now
  `require`s the appropriate precompiled module and calls its
  `parser_grammar()` constructor (cached per version) instead of reading
  and parsing a `.grammar` file from disk.
- The rockspec's `build.modules` table now lists every `_grammar_*.lua`
  submodule explicitly so `luarocks install` actually packages them.
- Public API (`parse`, `create_parser`, `get_grammar`), version
  validation, and error message text are unchanged (including the
  pre-existing error message text, which lists Java-style version
  numbers rather than the actual Haskell version set — left untouched
  since only the grammar-loading mechanism was in scope for this change).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of the grammar-driven Haskell parser.
- `M.parse(source, version)` — tokenizes Haskell source, loads
  `haskell/haskell<version>.grammar`, runs `GrammarParser`, and returns the root
  `ASTNode` (rule_name `"program"`).
- `M.create_parser(source, version)` — returns an initialized `GrammarParser`
  for manual control (e.g., trace-mode debugging).
- `M.get_grammar(version)` — exposes the cached `ParserGrammar` for inspection.
- Version routing: when `version` is `"1.0"`, `"1.1"`, `"1.4"`, `"5"`,
  `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, or `"21"`, the corresponding
  versioned grammar files are loaded from `code/grammars/haskell/`.
- Default version: passing `nil` or `""` defaults to Haskell 21.
- Per-version parser grammar cache keyed by version string.
- Validation: unknown version strings raise a descriptive error immediately.
- Grammar caching: grammar files are read from disk and parsed exactly once
  per process per version.
- Full test suite (`tests/test_haskell_parser.lua`) covering:
  - Module surface (VERSION, parse, create_parser, get_grammar)
  - Variable declarations
  - Assignments
  - Expression statements
  - Expression precedence
  - Multiple statements
  - Empty program
  - create_parser returns a usable parser
  - Version-aware parsing for all versions
  - Error handling
- `BUILD` and `BUILD_windows` with transitive dependency installation.
- `required_capabilities.json` declaring `filesystem:read`.
- `README.md` with architecture, grammar listing, and usage examples.
