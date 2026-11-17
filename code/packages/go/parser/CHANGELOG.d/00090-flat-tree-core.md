## [0.4.4] - 2026-11-17

Performance only (P2 PR 2): the engine core builds the tree in flat `int32`
arrays and `Parse` materialises `*ASTNode`s afterwards. No API change; trees,
positions, `Children` nil-ness, error messages, refusals, depth/frame
accounting, memo-slot use and the memo-hit node semantics are identical.

### Changed
- **Flat tree.** `match` pushes token indexes and `^node` references onto one
  pooled scratch stack; a finished rule copies its run into a per-parse `kids`
  array and appends a `cnode{rule, kids range, src, flags}`; memo slots store
  node indexes. Token index `len(tokens)` is the synthetic EOF. A failed
  alternative, a discarded lookahead or a superseded left-recursion seed now
  costs a few integer writes, not an `ASTNode`, a children slice and boxed
  tokens.
- **`toAST`.** After a successful parse, `Parse` sweeps the flat arrays
  (indexes go down from the root to mark reachable nodes, then up to build),
  iteratively, so tree depth cannot use the Go stack. It allocates only for
  reachable nodes: `[]ASTNode` chunks of at most 256, ONE `[]interface{}`
  children slab per parse (each node's run cap-clipped, so a consumer's
  `append` cannot overwrite a sibling), and each token position boxed at most
  once.
- **Legacy semantics kept on purpose** (`flagFromMemo`): a memo hit is a
  distinct `*ASTNode` that shares the first parse's `Children` slice and keeps
  zero `StartLine`/`EndLine`; `computeNodePosition` runs only on nodes that
  actually parsed, in children-first order; childless nodes get a non-nil
  empty `Children`.
- **New fail-closed limit.** The flat arrays are int32-indexed; running out of
  index space (never reachable in practice) fails the parse with
  `ErrMaxMemoSlots`, and a token list that long with `ErrMaxTokens`, instead of
  wrapping.
- **Bounded pools.** The stack, kids, nodes and toAST scratch are pooled with
  the memo slots and dropped on release when they exceed `maxPooledSlots`; the
  pointer-holding scratch is cleared so an idle state retains no tree or
  tokens. A parse that panics still never returns its state to the pool.

### Measured (median of 3-5 runs, M-series laptop, go1.26, vs. 0.4.3)

| benchmark | MB/s | B/op | allocs/op |
|---|---|---|---|
| JSONLike | 18.9 to 25.8 (1.36x) | 653,659 to 543,381 | 11,215 to 4,013 |
| Expression | 6.7 to 8.9 (1.32x) | 1,055,011 to 918,617 | 20,017 to 3,231 |
| SmallLine | 6.9 to 8.9 (1.29x) | 10,761 to 10,120 | 209 to 34 |
| StraceLike (real grammar, 5 lines) | 34.6 to 39.6 (1.15x) | 11,336 to 9,429 | 205 to 73 |
| JSONReal (400 records) | 23.7 to 34.0 (1.44x) | 2,622,163 to 2,124,351 | 44,816 to 16,039 |
| TypeScriptReal (ts5.8) | 1.73 to 2.05 (1.18x) | 795,940 to 700,027 | 15,473 to 2,224 |

### Tests
- Real-grammar goldens (`golden_test.go`, `testdata/golden/`): JSON and
  TypeScript 5.8 over real sources under default and tight depth, memo and
  token limits, with no trailing EOF and with truncated input; the strace
  corpus in `strace-parser`. Each golden is the SHA-256 of a canonical dump
  (tree, positions, nil-ness, tokens, shared-`Children` structure, or error
  text/kind/token) generated from 0.4.3 BEFORE this change. The existing
  frozen-engine differential, fuzzer, left-recursion knob differential and all
  limit/height tests pass unchanged.
- `flat_test.go`: memo-hit nodes (distinct, shared `Children`, zero
  positions, including a hit whose source was discarded by a lookahead and the
  left-recursion growth seed), non-nil empty `Children`, cap-clipped children
  (append on every node changes nothing else), a sharing-and-positions
  differential against the frozen original engine, pool reuse after success,
  syntax error, depth, memo and token limits, a panicking parse not returning
  its state to the pool, release clearing scratch and dropping oversized
  buffers, and int32 index overflow at all three growth sites.
- Real-grammar benchmarks `BenchmarkParseJSONReal`, `BenchmarkParseTypeScriptReal`.
