# Changelog

## [0.2.0] - 2026-03-23

### Added
- `GrammarDrivenParser` now accepts a `trace: false` keyword argument
- When `trace: true`, every call to `parse_rule` emits a `[TRACE]` line to `$stderr`
  with the format: `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail`
- Trace output uses the Unicode right arrow (→, U+2192) to separate context from outcome
- Both `match` and `fail` outcomes are reported, giving full visibility into backtracking
- Trace goes to `$stderr` (via `warn`) and is completely independent of stdout
- Test suite `test/test_trace.rb` with 16 tests covering:
  - Parse result identical with and without trace mode
  - Trace emitted to `$stderr` only (not stdout)
  - Correct line format (regex match on all lines)
  - Both `match` and `fail` outcomes present for a grammar with alternation
  - `trace: false` (default) produces no output

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
