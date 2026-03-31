# Changelog — coding-adventures-vhdl-parser

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of the VHDL parser (IEEE 1076-2008 synthesizable subset).
- Grammar-driven parsing using the `GrammarParser` engine from the `parser` package.
- Loads `code/grammars/vhdl.grammar` at runtime; caches after first load.
- `parse(source)` — tokenize VHDL source and return the AST root node.
- `create_parser(source)` — build a `GrammarParser` without immediately parsing.
- `get_grammar()` — expose the loaded `ParserGrammar` for inspection.
- Comprehensive `busted` test suite covering design files, entity declarations,
  architecture bodies, context clauses, concurrent/sequential statements,
  and expression grammar.
- Rockspec, BUILD, README, CHANGELOG, and `required_capabilities.json`.
