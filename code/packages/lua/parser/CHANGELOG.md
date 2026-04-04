# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-04

### Added

- ASTNode position fields: `start_line`, `start_column`, `end_line`, `end_column`
- Position computation from child tokens (first/last token span)
- GrammarParser handles four new element types:
  - `positive_lookahead` — match without consuming input
  - `negative_lookahead` — succeed if element does NOT match
  - `one_or_more` — one-or-more repetition
  - `separated_repetition` — separated list with optional at-least-one
- AST walking utilities:
  - `walk_ast(node, visitor)` — depth-first traversal with enter/leave callbacks
  - `find_nodes(node, rule_name)` — find all nodes matching a rule name
  - `collect_tokens(node, token_type)` — collect all leaf tokens, optionally filtered
  - `is_ast_node(child)` — check if a child is an ASTNode
- Helper functions: `_find_first_token`, `_find_last_token`

## [0.1.0] - 2026-03-23

### Added

- Hand-written recursive descent parser (Parser) with operator precedence
  - Parses expressions (addition, subtraction, multiplication, division)
  - Parses assignments (name = expression)
  - Parses parenthesized grouping
  - Supports number literals, string literals, and name identifiers
  - Left-associative operators, multiplication/division bind tighter than addition/subtraction
- AST node types: NumberLiteral, StringLiteral, NameNode, BinaryOp, Assignment, ExpressionStmt, Program
- Grammar-driven packrat parser (GrammarParser)
  - Interprets BNF-like grammar rules at runtime
  - Packrat memoization for O(n * g) performance
  - Supports: sequence, alternation, repetition, optional, group, literal, rule/token references
  - Auto-detects newline significance from grammar rules
  - Furthest-failure error reporting for clear parse error messages
  - Optional trace mode for debugging grammar problems
- ASTNode for grammar-driven parse trees with is_leaf() and token() helpers
- ParseError and GrammarParseError with line:column error formatting
- Token type constants mirroring the Go lexer TokenType enum
- token_type_name() utility for both string-based and enum-based token matching
- 84 busted tests covering all parser modes, AST nodes, error handling, and edge cases
- Ported from the Go implementation at code/packages/go/parser/
