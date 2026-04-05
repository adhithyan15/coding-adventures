# Changelog — coding-adventures-typescript-parser

All notable changes to this package are documented here.

## [0.2.0] — 2026-04-05

### Added

- `version` parameter added to `parse(source, version)`,
  `create_parser(source, version)`, and `get_grammar(version)`.
- Version routing: when `version` is `"ts1.0"`, `"ts2.0"`, `"ts3.0"`,
  `"ts4.0"`, `"ts5.0"`, or `"ts5.8"`, the corresponding versioned grammar
  files are loaded from `code/grammars/typescript/<version>.grammar` and the
  lexer uses `code/grammars/typescript/<version>.tokens`.
- Generic fallback: passing `nil` or `""` loads the unified grammars as
  before (backward compatible).
- Per-version parser grammar cache keyed by version string.
- Validation: unknown version strings raise a descriptive error immediately.
- Extended test suite: new `describe("version-aware parsing")` block
  covering all 6 recognized versions, `create_parser`, `get_grammar`, and
  error cases for invalid versions.

### Changed

- `M.VERSION` bumped from `"0.1.0"` to `"0.2.0"`.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of the TypeScript parser.
- Grammar-driven parsing using the `GrammarParser` engine from the `parser` package.
- Loads `code/grammars/typescript.grammar` at runtime; caches after first load.
- `parse(source)` — tokenize TypeScript and return the AST root node.
- `create_parser(source)` — build a `GrammarParser` without immediately parsing.
- `get_grammar()` — expose the loaded `ParserGrammar` for inspection.
- Comprehensive `busted` test suite covering all grammar constructs.
- Rockspec, BUILD, README, CHANGELOG, and `required_capabilities.json`.
