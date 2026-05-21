# Changelog

All notable changes to the TOML Parser package will be documented in this file.

## [0.1.1] — 2026-05-21

### Changed

- **Use the precompiled grammar at runtime instead of `readFileSync`-ing
  `toml.grammar`.**  `src/_grammar.ts` (auto-generated from
  `code/grammars/toml.grammar` via `grammar-tools compile-grammar`) was
  already present in the repo but unused — `parser.ts` was still loading
  the grammar text from disk and re-parsing it on every `parseTOML` call.
  We now import `PARSER_GRAMMAR` directly from `_grammar.ts`.

  **Capability impact**: `required_capabilities.json` drops from
  `[{ fs:read → ../../grammars/toml.grammar }]` to `[]`.  Every
  downstream consumer that imports `@coding-adventures/toml-parser`
  now keeps its own `[]` capabilities (previously, any TS package
  that called `parseTOML` transitively required `fs:read`).

  **Performance impact**: first-call latency improves (no fs round-trip,
  no grammar text re-parsing).  Subsequent calls are unchanged.

  **No API change**: `parseTOML(source)` returns the same `ASTNode`.
  All 34 existing tests pass with no modifications.

### Removed

- `fileURLToPath`, `dirname`, `join`, `readFileSync` imports
  (no longer needed).
- The `GRAMMARS_DIR` / `TOML_GRAMMAR_PATH` constants.
- The `parseParserGrammar` import (the grammar arrives pre-parsed).

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TOML parser.
- `parseTOML()` function that parses TOML text into ASTs using the grammar-driven parser engine.
- Loads `toml.grammar` file defining all TOML v1.0.0 syntax rules.
- Supports all TOML value types: strings (4 types), integers, floats, booleans, date/time (4 types), arrays, inline tables.
- Supports table headers ([table]) and array-of-tables headers ([[array]]).
- Supports dotted keys (a.b.c = 1) and quoted keys ("127.0.0.1" = value).
- Multi-line array parsing with optional trailing commas.
- Comprehensive test suite covering primitive values, key-value pairs, table headers, array-of-tables, arrays, inline tables, complete documents, and error cases.
