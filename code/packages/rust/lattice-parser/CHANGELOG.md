# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of the Lattice CSS superset parser
- `parse_lattice(source: &str) -> GrammarASTNode` — parse Lattice source to an AST (panics on syntax error)
- `create_lattice_parser(source: &str) -> GrammarParser` — create a reusable parser instance
- Re-export of `ASTNodeOrToken` for downstream crates
- Grammar-driven parsing via `lattice.grammar` grammar file read at runtime
- Full AST support for Lattice constructs: `variable_declaration`, `mixin_definition`, `include_directive`, `if_directive`, `for_directive`, `each_directive`, `function_definition`, `return_directive`, `use_directive`
- Full CSS support: `qualified_rule`, `at_rule`, `selector_list`, `complex_selector`, `compound_selector`, `class_selector`, `id_selector`, `pseudo_class`, `pseudo_element`, `attribute_selector`, `declaration`, `value_list`
- 20 unit tests with recursive `find_rule` helper to verify grammar rule presence in AST
