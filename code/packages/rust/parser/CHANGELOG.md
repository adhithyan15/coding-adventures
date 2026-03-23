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

## [0.1.1] - 2026-03-23

### Fixed

- **`match_token_reference` custom type disambiguation**: tokens with a `type_name` set (e.g. `IDENT`, `VARIABLE`, `FUNCTION`) would previously match any token reference whose grammar name maps to `TokenType::Name` as a fallback — because `string_to_token_type` returns `Name` for unknown names. For example, an `IDENT` token would match a `VARIABLE` reference even though they are different grammar-level types. The fix: when `expected_type` maps to `Name` but is not literally `"NAME"`, and the current token already has a specific `type_name`, reject the match unless `type_name == expected_type`. This enables grammar rules like `rule = at_rule | qualified_rule` to correctly dispatch on the leading token type rather than collapsing all `Name`-typed tokens into the first alternative.
