## [0.3.0] - 2026-04-04

### Added
- **AST position fields**: `ASTNode` now has `StartLine`, `StartColumn`, `EndLine`,
  `EndColumn` fields (int, 0 = unset). Positions are automatically computed from the
  first and last leaf tokens in the children tree.
- **Positive lookahead matching**: `matchElement` handles `PositiveLookahead` — matches
  without consuming input if inner element succeeds.
- **Negative lookahead matching**: `matchElement` handles `NegativeLookahead` — succeeds
  without consuming input if inner element does NOT match.
- **One-or-more repetition matching**: `matchElement` handles `OneOrMoreRepetition` —
  like zero-or-more but requires at least one match.
- **Separated repetition matching**: `matchElement` handles `SeparatedRepetition` —
  matches `element { separator element }` with optional at-least-one constraint.
- **`WalkAST(node, visitor)`**: Depth-first walk with enter/leave callbacks that can
  replace nodes. Token children are not visited.
- **`FindNodes(node, ruleName)`**: Returns all nodes matching a rule name in depth-first
  order.
- **`CollectTokens(node, tokenType)`**: Collects all leaf tokens in depth-first order,
  optionally filtered by type name.
- `ASTVisitor` struct with optional `Enter` and `Leave` callbacks.
- `elementReferencesNewline` handles all four new element types.
- Internal helpers `computeNodePosition`, `findFirstToken`, `findLastToken`.

