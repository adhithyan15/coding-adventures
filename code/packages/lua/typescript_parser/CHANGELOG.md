# Changelog — coding-adventures-typescript-parser

All notable changes to this package are documented here.

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
