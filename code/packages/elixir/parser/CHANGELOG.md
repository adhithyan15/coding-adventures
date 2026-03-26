# Changelog

## 0.1.0 — 2026-03-20

### Added
- `GrammarParser.parse/2` — grammar-driven parser engine with backtracking
- `ASTNode` struct with `rule_name` and `children` fields
- Packrat memoization for O(n × R) performance
- Support for all EBNF elements: sequence, alternation, repetition, optional, group, rule reference, literal
- Newline significance auto-detection
- Furthest-failure error reporting
- JSON grammar integration tests (10 tests)
