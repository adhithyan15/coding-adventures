# Changelog — coding-adventures-haskell-parser

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
