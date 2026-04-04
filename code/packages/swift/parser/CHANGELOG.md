# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added
- `ASTNode` struct with `ruleName`, `children`, and position fields (`startLine`, `startColumn`, `endLine`, `endColumn`)
- `ASTChild` enum (`.node(ASTNode)` / `.token(Token)`) for type-safe AST children
- Helper functions: `isASTNode()`, `isLeafToken()`, `isLeafNode()`, `getLeafToken()`
- `GrammarParser` class with packrat memoization and Warth left-recursion support
- Support for all EBNF element types:
  - Sequence, alternation, repetition, optional, group, literal
  - Token reference, rule reference
  - **New**: `.positiveLookahead` (`& element`)
  - **New**: `.negativeLookahead` (`! element`)
  - **New**: `.oneOrMore` (`element +`)
  - **New**: `.separatedRepetition` (`element // separator`)
- Pre-parse hooks for token stream transforms
- Post-parse hooks for AST transforms
- Furthest-failure error reporting with `GrammarParseError`
- Automatic newline significance detection
- **New**: `ASTWalker` module with utilities:
  - `walkAST()` -- depth-first pre-order traversal
  - `findNodes(in:where:)` -- search by predicate
  - `findNodes(in:named:)` -- search by rule name
  - `collectTokens(from:)` -- gather all leaf tokens
- Comprehensive test suite with 30+ test cases
