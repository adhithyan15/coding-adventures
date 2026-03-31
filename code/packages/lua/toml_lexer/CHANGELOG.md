# Changelog — coding-adventures-toml-lexer (Lua)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the TOML lexer Lua package.
- `tokenize(source)` — tokenizes a TOML source string using the grammar-driven
  `GrammarLexer` infrastructure, returning a flat list of typed tokens.
- `get_grammar()` — returns the cached `TokenGrammar` object parsed from
  `toml.tokens`, for callers that want to inspect or reuse the grammar.
- Grammar loading with caching — the `toml.tokens` file is read and parsed once
  per process; subsequent calls reuse the cached `TokenGrammar`.
- Path navigation — locates `toml.tokens` by walking 6 directories up from the
  module file to the `code/` repo root, then descending into `grammars/`.
- Full test suite (`tests/test_toml_lexer.lua`) covering:
  - Module surface (VERSION, tokenize, get_grammar)
  - Empty and whitespace-only inputs
  - Key-value pairs with BARE_KEY, EQUALS, BASIC_STRING
  - Table headers `[section]` and `[[array-of-tables]]`
  - All four TOML string types (basic, literal, multi-line basic, multi-line literal)
  - All integer forms (decimal, hex `0x`, octal `0o`, binary `0b`, underscore-separated)
  - All float forms (decimal, scientific, `inf`, `-inf`, `nan`)
  - Boolean literals (`true`, `false`)
  - All date/time types (OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME)
  - Inline tables `{ key = "val" }`
  - Arrays `[1, 2, 3]`
  - Whitespace and comment consumption
  - Token position tracking (line, col)
  - Error on unexpected character
- `coding-adventures-toml-lexer-0.1.0-1.rockspec` rockspec with correct
  transitive dependencies (state-machine, directed-graph, grammar-tools, lexer).
- `BUILD` and `BUILD_windows` scripts installing all dependencies leaf-to-root.
- `required_capabilities.json` declaring `filesystem:read` capability.
