# Changelog

## [0.3.0] - 2026-04-04

### Added
- **ASTNode positions**: `start_line`, `start_column`, `end_line`, `end_column`
  fields computed from child tokens. Nil when the node has no tokens (e.g.,
  empty repetition). Downstream tools can map AST nodes to source locations.
- **GrammarParser**: `match_element` handles new grammar element types:
  - `PositiveLookahead` -- succeed without consuming if inner matches
  - `NegativeLookahead` -- succeed without consuming if inner fails
  - `OneOrMoreRepetition` -- match one then zero-or-more additional
  - `SeparatedRepetition` -- match elements with separators between them
- **AST walking utilities** (module-level methods on `CodingAdventures::Parser`):
  - `walk_ast(node, visitor)` -- depth-first traversal with enter/leave callbacks
  - `find_nodes(node, rule_name)` -- find all nodes matching a rule name
  - `collect_tokens(node, type = nil)` -- collect tokens, optionally by type

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
