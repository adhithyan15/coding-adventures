# Changelog

## 0.2.0 — 2026-04-04

### Added
- `ASTNode` position fields: `start_line`, `start_column`, `end_line`, `end_column` —
  computed from the first and last leaf tokens in the children tree
- `ASTNode.ast_node?/1` — check if a child element is an ASTNode (not a Token)
- `ASTNode.walk_ast/2` — depth-first tree traversal with `:enter`/`:leave` visitor callbacks
- `ASTNode.find_nodes/2` — find all nodes matching a rule name
- `ASTNode.collect_tokens/2` — collect all tokens in depth-first order, optionally by type
- `GrammarParser` match_element clauses for new grammar element types:
  - `{:positive_lookahead, element}` — succeed if element matches, consume no input
  - `{:negative_lookahead, element}` — succeed if element does NOT match, consume no input
  - `{:one_or_more, element}` — match one required, then zero or more additional
  - `{:separated_repetition, element, separator, at_least_one}` — element { sep element }
- Newline significance detection handles all new element types

## 0.1.0 — 2026-03-20

### Added
- `GrammarParser.parse/2` — grammar-driven parser engine with backtracking
- `ASTNode` struct with `rule_name` and `children` fields
- Packrat memoization for O(n × R) performance
- Support for all EBNF elements: sequence, alternation, repetition, optional, group, rule reference, literal
- Newline significance auto-detection
- Furthest-failure error reporting
- JSON grammar integration tests (10 tests)
