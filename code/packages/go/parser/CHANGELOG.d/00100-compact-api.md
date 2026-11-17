## [0.5.0] - 2026-11-17

Adds a compact API (P2 PR 3). `Parse` is unchanged: same trees, positions,
errors, limits and performance.

### Added
- **`Tree` and `(*Compiled).ParseInto(t, tokens, opts) error`.** Runs the same
  engine as `Parse` and leaves the flat result (`nodes`, `kids`, a reference to
  the token slice) in a reusable `Tree` instead of building `*ASTNode`s. With
  one Tree reused, steady-state parses allocate nothing.
- **Accessors:** `Root`, `Rule`, `RuleID`, `NumChildren`, `Child` (type `Child`:
  `IsToken`, `Token`, `Node`), `Token`, `NumTokens`, `NumNodes`, `Span`,
  `Position`, `IsReplay`, `Reset`; `Compiled.RuleID`, `Compiled.RuleName`;
  `NodeID`, `NoNode`.
- **`(*Tree).ToAST()`** rebuilds exactly the tree `Parse` returns (structure,
  positions, nil-ness, node and `Children` sharing). It shares `buildAST`
  with `Parse`, which is why the identity golden passes unchanged through it.

### Design choices
- **Memo hits.** A replay is a distinct `NodeID` (`IsReplay`) that shares the
  replayed node's children; `Position` is zero for it (as the `ASTNode` is),
  while `Span` reports the real token range.
- **Ownership.** `ParseInto` swaps buffers with the pooled state instead of
  copying, so a Tree never aliases engine state. It keeps a reference to the
  caller's tokens (after `PreParse` hooks), uncopied. Not safe for concurrent
  use; distinct Trees are independent.
- **Empty on error.** Any error, limit refusal or recovered panic leaves the
  Tree empty (`Root() == NoNode`, `ToAST() == nil`).
- **Bounded reuse.** A Tree keeps at most `maxPooledFlat` entries per buffer
  between parses.
- **`PostParse` is refused** with an error: it transforms `*ASTNode`s that
  `ParseInto` never builds.

### Internal
- `Parse` split into `prepare` (hooks and pre-engine refusals), `runFlat`
  (engine, no `toAST`) and `buildAST` (`toAST` over an explicit flat tree with
  an `astScratch`). Behaviour identical.

### Measured (M-series laptop, noisy machine, go1.26)

| benchmark | Parse ns/op, B/op, allocs | ParseInto ns/op, B/op, allocs |
|---|---|---|
| JSONLike | 322,000 to 330,000, 543,146 B, 4,013 | about 152,000, 73 B, 0 |
| Expression | 670,000 to 690,000, 915,065 B, 3,230 | about 395,000, about 400 B, 0 |
| SmallLine | 6,830 to 7,000, 10,121 B, 34 | about 4,100, 0 B, 0 |
| StraceLike (5 lines) | 9,500, 9,429 B, 73 | about 6,000, 0 B, 0 |

### Tests
- `tree_test.go`: limits (MaxDepth, MaxMemoSlots, MaxTokens, frame budget,
  height, flat index overflow) swept through both paths with identical
  outcomes; random grammars with one reused Tree; memo-hit accessor semantics;
  reuse (big, small, error, small) with capacity caps; zero allocations in
  steady state; no aliasing of pooled state; concurrent distinct Trees
  (`-race`); misuse panics; edge cases (nil, zero, hooks, panics).
- `differential_test.go` (and so `FuzzDifferential`) is now three-way: frozen
  old engine, `Parse`, and `ParseInto` with `ToAST` plus a walk using only
  the public accessors, into fresh and reused Trees.
- `TestIdentityGoldenParseInto` runs the 200,000-parse identity golden through
  `ParseInto` and `ToAST` against the same committed hashes.
