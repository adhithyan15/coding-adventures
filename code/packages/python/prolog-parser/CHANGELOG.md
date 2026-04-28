# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-23

### Added

- first executable Prolog parser package
- `code/grammars/prolog.grammar` as the parser syntax source of truth
- `create_prolog_parser`, `parse_ast`, `parse_source`, `parse_program`, and
  `parse_query`
- `lower_ast` so dialect-specific parser packages can reuse executable lowering
- lowering for facts, rules, queries, terms, lists, conjunction, disjunction,
  cut, unification, and disequality
- end-to-end tests proving parsed source executes through `logic-engine`
