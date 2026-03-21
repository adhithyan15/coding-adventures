# Changelog

All notable changes to the `parser` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `ast` module with `ASTNode` enum: `Number`, `String`, `Name`, `BinaryOp`, `Assignment`, `ExpressionStmt`, `Program`.
- `parser` module with hand-written recursive descent parser for a Python subset:
  - Arithmetic expressions with operator precedence (`*`/`/` before `+`/`-`).
  - Parenthesized sub-expressions.
  - Variable assignments (`x = expr`).
  - Multi-statement programs with newline separation.
  - `Result`-based error handling with `ParseError` type.
- `grammar_parser` module with grammar-driven parser:
  - `GrammarParser` that reads rules from a `ParserGrammar` (from `grammar-tools`).
  - Backtracking support for alternation.
  - Handles Sequence, Alternation, Repetition, Optional, Group, RuleReference, TokenReference, and Literal grammar elements.
  - `GrammarASTNode` with `rule_name` and `children` (either nested nodes or tokens).
  - `is_leaf()` and `token()` helper methods on `GrammarASTNode`.
- Comprehensive test suite covering:
  - Expression parsing (addition, multiplication, precedence, parentheses).
  - Statement parsing (assignments, expression statements).
  - Multi-statement programs and blank line handling.
  - Error cases (unexpected tokens).
  - Grammar-driven parsing (single values, addition, chaining, alternation, optional, literals, groups).
  - Integration tests using the lexer to tokenize source code before parsing.
