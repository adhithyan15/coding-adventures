# Changelog

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

## [0.2.1] - 2026-04-02

### Fixed
- Added `.PanicOnUnexpected()` to `Parse` so intentional panics (unexpected token, malformed input) propagate correctly instead of being swallowed by the Operations panic-recovery wrapper.

## [0.2.0] - Unreleased

### Added

- `trace bool` field on `GrammarParser` struct.
- `NewGrammarParserWithTrace(tokens []Token, grammar *ParserGrammar, trace bool) *GrammarParser` constructor.
  When `trace=true`, each rule attempt is printed to stderr:
  `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match`
  `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → fail`
  Packrat memoization means each `(rule, position)` pair is traced at most once.
- `NewGrammarParser` now delegates to `NewGrammarParserWithTrace(..., false)`.
- Trace mode tests: `TestGrammarParserWithTraceNoError`, `TestGrammarParserWithTraceMatchesNoTrace`, `TestGrammarParserWithTraceFailurePath`.

## [0.1.0] - Unreleased

### Added
- Parser LL(2) structs navigating `Tokens` natively into explicit AST structs mappings. 
- Integrated native Parser structure extracting syntax bindings recursively resolving mathematical orders without generic limits.
