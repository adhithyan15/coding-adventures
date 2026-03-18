# Changelog

## [0.1.0] - 2026-03-18

### Added
- `RecursiveDescentParser` class -- hand-written recursive descent parser
- `GrammarDrivenParser` class -- grammar-driven parser that reads .grammar files
- AST nodes: `NumberLiteral`, `StringLiteral`, `Name`, `BinaryOp`, `Assignment`, `Program`
- `ASTNode` generic node type for grammar-driven parsing
- Operator precedence: *, / before +, -
- Left-associative binary operations
- Parenthesized expression support
- `ParseError` and `GrammarParseError` with token location info
