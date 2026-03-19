# Changelog

All notable changes to the `@coding-adventures/parser` package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- **Hand-written recursive descent parser** (`Parser` class)
  - Parses token streams into typed AST nodes (`NumberLiteral`, `StringLiteral`, `Name`, `BinaryOp`, `Assignment`, `Program`)
  - Implements operator precedence via grammar rule nesting (multiplication/division before addition/subtraction)
  - Left-associative operator parsing
  - Parenthesized expression support for precedence override
  - Assignment statement parsing with LL(2) lookahead
  - Expression statement parsing
  - `ParseError` class with token location information

- **Grammar-driven parser** (`GrammarParser` class)
  - Interprets EBNF grammar rules from `.grammar` files at runtime
  - Produces generic `ASTNode` trees (language-agnostic)
  - Supports all EBNF constructs: Sequence, Alternation, Repetition, Optional, Group, TokenReference, RuleReference, Literal
  - Backtracking for alternation handling
  - Automatic newline skipping within expressions
  - `GrammarParseError` class with optional token location
  - Helper functions: `isASTNode()`, `isLeafNode()`, `getLeafToken()`

- **Full test suite** with >80% coverage
  - Hand-written parser tests: atoms, binary ops, precedence, parentheses, assignments, multiple statements, error handling, end-to-end
  - Grammar-driven parser tests: same coverage using actual `python.grammar` file

- Knuth-style literate programming throughout all source files
- TypeScript port from Python `lang_parser` package
