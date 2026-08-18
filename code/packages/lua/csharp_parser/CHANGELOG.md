# Changelog — coding-adventures-csharp-parser (Lua)

All notable changes to this package are documented here.

## [Unreleased]

### Fixed

- Eliminated runtime grammar loading. Previously, `get_grammar(version)`
  read `code/grammars/csharp/csharp<version>.grammar` off disk at runtime
  using a path that walked outside this package's own directory into the
  monorepo (`debug.getinfo`-based directory walk-up). That works inside a
  checkout of the monorepo, but a published LuaRocks package does not
  include `code/grammars/` — installing this rock and calling
  `parse_csharp` would raise a file-not-found error.
- Each of the 12 supported C# versions' `.grammar` file is now compiled
  ahead of time (via `grammar-tools compile-grammar`) into a
  `_grammar_<version>.lua` sibling module (e.g. `_grammar_12_0.lua`) that
  embeds the parsed `ParserGrammar` as native Lua data. `init.lua` now
  `require`s the appropriate precompiled module and calls its
  `parser_grammar()` constructor (cached per version) instead of reading
  and parsing a `.grammar` file from disk.
- The rockspec's `build.modules` table now lists every `_grammar_*.lua`
  submodule explicitly so `luarocks install` actually packages them.
- Public API (`parse_csharp`, `create_csharp_parser`, `get_grammar`),
  version validation, and error message text are unchanged.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.csharp_parser`.
- `parse_csharp(source, version)` — parses a C# string using the shared
  `csharp/csharp<version>.grammar` specification and the grammar-driven
  `GrammarParser` from `coding-adventures-parser`. Returns the root `ASTNode`
  with `rule_name == "program"`.
- `create_csharp_parser(source, version)` — tokenizes the source and returns an
  initialized `GrammarParser` without immediately parsing (useful for deferred
  or incremental parsing workflows).
- `get_grammar(version)` — returns the cached `ParserGrammar` for direct
  inspection or reuse.
- Version routing: when `version` is `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`,
  `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, or
  `"12.0"`, the corresponding versioned grammar file is loaded from
  `code/grammars/csharp/csharp<version>.grammar`.
- Default version: passing `nil` or `""` defaults to C# 12.0.
- Per-version grammar cache keyed by version string.
- Validation: unknown version strings raise a descriptive error immediately.
- Comprehensive busted test suite covering module surface, root node shape,
  variable declarations, assignments, expression statements, operator
  precedence, parenthesized expressions, multiple statements, empty programs,
  `create_csharp_parser`, `get_grammar`, version-aware parsing (all 12
  versions), cache behavior, and error cases.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order.
