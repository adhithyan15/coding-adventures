# Changelog — coding-adventures-haskell-lexer (Lua)

All notable changes to this package are documented here.

## [Unreleased]

### Fixed

- Eliminated runtime grammar loading. Previously, `get_grammar(version)`
  read `code/grammars/haskell/haskell<version>.tokens` off disk at
  runtime using a path that walked outside this package's own directory
  into the monorepo (`debug.getinfo`-based directory walk-up). That works
  inside a checkout of the monorepo, but a published LuaRocks package
  does not include `code/grammars/` — installing this rock and calling
  `tokenize` would raise a file-not-found error.
- Each of the 7 supported Haskell versions' `.tokens` grammar
  (`1.0`, `1.1`, `1.2`, `1.3`, `1.4`, `98`, `2010`) is now compiled ahead
  of time (via `grammar-tools compile-tokens`) into a
  `_grammar_<version>.lua` sibling module (e.g. `_grammar_2010.lua`) that
  embeds the parsed `TokenGrammar` as native Lua data. `init.lua` now
  `require`s the appropriate precompiled module and calls its
  `token_grammar()` constructor (cached per version) instead of reading
  and parsing a `.tokens` file from disk.
- The rockspec's `build.modules` table now lists every `_grammar_*.lua`
  submodule explicitly so `luarocks install` actually packages them.
- Public API (`tokenize`, `create_lexer`, `get_grammar`), version
  validation, and error message text are unchanged (including the
  pre-existing error message text, which lists Java-style version
  numbers rather than the actual Haskell version set — left untouched
  since only the grammar-loading mechanism was in scope for this change).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `coding_adventures.haskell_lexer`.
- `tokenize(source, version)` — tokenizes a Haskell string using the shared
  `haskell/haskell<version>.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `create_lexer(source, version)` — returns an initialized `GrammarLexer`
  without immediately tokenizing.
- `get_grammar(version)` — returns the cached `TokenGrammar` for direct use.
- Version routing: when `version` is `"1.0"`, `"1.1"`, `"1.4"`, `"5"`,
  `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, or `"21"`, the corresponding
  versioned grammar file is loaded from `code/grammars/haskell/haskell<version>.tokens`.
- Default version: passing `nil` or `""` defaults to Haskell 21.
- Per-version grammar cache keyed by version string.
- `\v` and `\f` escape normalization applied when loading grammar files
  (Lua's regex engine requires literal control characters instead).
- Validation: unknown version strings raise a descriptive error immediately.
- Comprehensive busted test suite covering module surface, empty/trivial
  inputs, identifiers, numbers, strings, punctuation, whitespace handling,
  position tracking, version-aware tokenization, cache behavior, and error
  cases.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation.
