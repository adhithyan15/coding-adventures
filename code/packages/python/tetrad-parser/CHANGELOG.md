# Changelog — tetrad-parser

## [0.1.0] — 2026-04-20

### Added

- `tetrad_parser.ast` module: 18 dataclass node types covering the full Tetrad grammar
  - Root: `Program`
  - Declarations: `FnDecl` (with `param_types`, `return_type` for gradual typing), `GlobalDecl`
  - Statements: `Block`, `LetStmt`, `AssignStmt`, `IfStmt`, `WhileStmt`, `ReturnStmt`, `ExprStmt`
  - Expressions: `IntLiteral`, `NameExpr`, `BinaryExpr`, `UnaryExpr`, `CallExpr`, `InExpr`, `OutExpr`, `GroupExpr`
  - Union type aliases: `Expr`, `Stmt`
- `_Parser` class implementing Pratt expression parser + recursive-descent statement/declaration parser
- Binding power table for all 18 binary operators with correct precedence
- Two-token lookahead to distinguish `IDENT = expr` (assignment) from `IDENT == expr` (comparison expression)
- `parse_type()` with helpful error messages for unknown type names vs. missing types
- `parse(source: str) -> Program` public entry point
- `ParseError` exception with `.message`, `.line`, `.column`
- 90+ unit tests covering all node types, precedence rules, error paths, and the TET00 example programs
- 95%+ line coverage
