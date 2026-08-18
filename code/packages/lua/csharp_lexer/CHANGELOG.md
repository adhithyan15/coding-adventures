# Changelog — coding-adventures-csharp-lexer (Lua)

All notable changes to this package are documented here.

## [Unreleased]

### Fixed

- Eliminated runtime grammar loading. Previously, `get_grammar(version)`
  read `code/grammars/csharp/csharp<version>.tokens` off disk at runtime
  using a path that walked outside this package's own directory into the
  monorepo (`debug.getinfo`-based directory walk-up). That works inside a
  checkout of the monorepo, but a published LuaRocks package does not
  include `code/grammars/` — installing this rock and calling
  `tokenize_csharp` would raise a file-not-found error.
- Each of the 12 supported C# versions' `.tokens` grammar is now compiled
  ahead of time (via `grammar-tools compile-tokens`) into a
  `_grammar_<version>.lua` sibling module (e.g. `_grammar_12_0.lua`) that
  embeds the parsed `TokenGrammar` as native Lua data. `init.lua` now
  `require`s the appropriate precompiled module and calls its
  `token_grammar()` constructor (cached per version) instead of reading
  and parsing a `.tokens` file from disk.
- The rockspec's `build.modules` table now lists every `_grammar_*.lua`
  submodule explicitly so `luarocks install` actually packages them —
  without this, the compiled grammar files, while present in the source
  tree, would not be installed alongside `init.lua`, silently reproducing
  the same file-not-found failure via a different missing-file path.
- Public API (`tokenize_csharp`, `create_csharp_lexer`, `get_grammar`),
  version validation, and error message text are unchanged.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.csharp_lexer`.
- `tokenize_csharp(source, version)` — tokenizes a C# string using the shared
  `csharp/csharp<version>.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `create_csharp_lexer(source, version)` — returns an initialized `GrammarLexer`
  without immediately tokenizing.
- `get_grammar(version)` — returns the cached `TokenGrammar` for direct use.
- Version routing: when `version` is `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`,
  `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, or `"12.0"`,
  the corresponding versioned grammar file is loaded from
  `code/grammars/csharp/csharp<version>.tokens`.
- Default version: passing `nil` or `""` defaults to C# 12.0.
- Per-version grammar cache keyed by version string.
- `\v` and `\f` escape normalization applied when loading grammar files
  (Lua's regex engine requires literal control characters instead).
- Validation: unknown version strings raise a descriptive error immediately.
- Comprehensive busted test suite covering module surface, empty/trivial
  inputs, identifiers, numbers, strings, punctuation, whitespace handling,
  position tracking, version-aware tokenization (all 12 versions), cache
  behavior, and error cases.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation.
