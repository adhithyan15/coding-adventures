# Changelog — coding-adventures-verilog-parser

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of the Verilog parser (IEEE 1364-2005 synthesizable subset).
- Grammar-driven parsing using the `GrammarParser` engine from the `parser` package.
- Loads `code/grammars/verilog.grammar` at runtime; caches after first load.
- `parse(source)` — tokenize Verilog source and return the AST root node.
- `create_parser(source)` — build a `GrammarParser` without immediately parsing.
- `get_grammar()` — expose the loaded `ParserGrammar` for inspection.
- Comprehensive `busted` test suite covering module declarations, port lists,
  wire/reg declarations, continuous assignments, always blocks, if/case/for
  statements, module instantiation, and expression grammar.
- Rockspec, BUILD, README, CHANGELOG, and `required_capabilities.json`.
