# Changelog

All notable changes to the `parser` crate will be documented in this file.

## [0.3.0] - 2026-04-04

### Added
- `GrammarASTNode` position fields: `start_line`, `start_column`,
  `end_line`, `end_column` (all `Option<usize>`) — computed from the
  first and last leaf tokens in the node's children.
- `compute_node_position`, `find_first_token`, `find_last_token` —
  helper functions for AST node position computation.
- `ASTVisitor` trait with `enter`/`leave` callbacks for AST traversal.
- `walk_ast(node, visitor)` — depth-first walk with enter/leave phases;
  visitor callbacks can return replacement nodes.
- `find_nodes(node, rule_name)` — collect all nodes matching a rule name.
- `collect_tokens(node, type_filter)` — collect all tokens in depth-first
  order, optionally filtered by type name.
- `match_element` arms for new `GrammarElement` variants:
  - `PositiveLookahead` — succeeds without consuming input if inner matches.
  - `NegativeLookahead` — succeeds without consuming input if inner fails.
  - `OneOrMore` — matches one required then zero or more additional.
  - `SeparatedRepetition` — matches element { separator element } pattern.
- `element_references_newline` updated for new variants.
- New exports from `lib.rs`: `ASTNodeOrToken`, `ASTVisitor`, `walk_ast`,
  `find_nodes`, `collect_tokens`.

## [0.2.0] - 2026-03-23

### Added

- `GrammarParser::new_with_trace(tokens, grammar, trace: bool)` constructor
  - When `trace = true`, emits a `[TRACE]` line to stderr for every grammar
    rule attempt, showing the rule name, token index, token type and value,
    and whether the rule matched or failed
  - Format: `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail`
  - Trace output goes to stderr so it does not pollute parser return values
  - `new()` is now a thin wrapper over `new_with_trace(..., false)` (no behaviour change)
- Added `trace: bool` field to `GrammarParser` struct
- 4 new unit tests for trace mode in `grammar_parser::tests`:
  - `test_trace_mode_parse_succeeds` — trace does not affect parse correctness
  - `test_trace_mode_no_panic_on_failure` — trace does not panic on bad input
  - `test_trace_mode_addition` — multi-token sequence works in trace mode
  - `test_trace_false_same_as_new` — `new_with_trace(false)` == `new()`

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
