## [0.4.1] - 2026-11-16

Hardening from the G2 security and design reviews.

### Security
- **Left recursion no longer bypasses `MaxDepth`.** Seed growing is a loop, so
  call depth stayed at 1 while the tree grew one level per term; position
  computation then recursed down the left spine and `e = e PLUS NUMBER` with
  about 7 million terms ended in `fatal error: stack overflow`. Every
  successful growth step now consumes one level of the depth budget, so tree
  height is a bounded invariant (a chain of more than about `MaxDepth`
  left-associative terms, default 10,000, is refused with `ErrMaxDepth`).
- `computeNodePosition`, `findFirstToken`, `findLastToken`, `WalkAST`,
  `FindNodes` and `CollectTokens` are iterative, so they are safe on a tree of
  any depth, including trees callers build themselves. Position computation is
  also O(1) per node on first-parse trees (it used to re-walk the spine).
- **`HardMaxDepth` is now a real stack guarantee.** Only rule calls were
  counted; matcher recursion through nested groups, optionals, repetitions and
  lookaheads was not (about 50 nested groups at the clamp overflowed). Each
  rule is now charged its element nesting, and the running total is capped at
  `HardMaxFrames` (200,000, about 64 MB of stack in the worst case measured).
  The depth comments are corrected: a goroutine's real maximum stack is
  512 MB, not 1 GB, because stacks double and 1,024 MB exceeds the 1e9 cap.
- `Options.MaxTokens` (`ErrMaxTokens`) and `Options.MaxMemoSlots`
  (`ErrMaxMemoSlots`) cap the memory an untrusted input can cost. The memo slot
  index is also checked against `math.MaxInt32`.

### Fixed
- A panic in an `AddPreParse` hook escaped `GrammarParser.Parse`; it is now
  returned as an error like every other hook panic (regression against main).
- Memo-hit nodes for rules that matched nothing had `Children == nil` where the
  first parse gave an empty slice (JSON `[]` became `null`, `reflect.DeepEqual`
  failed). Children are now normalised before they are memoised.

### Changed
- `NewGrammarParser*` cache the compiled grammar per `*ParserGrammar` pointer
  (bounded to 512 grammars). Small inputs against a large grammar were 3x
  slower than the old engine because every constructor recompiled; ts5.8
  `let x = 1 + 2;` is now about 3.5x faster than the old engine instead
  (88 us to 25 us per parse, 905 to 170 allocations). A grammar must not be
  mutated after first use; `ForgetGrammar(g)` drops its cache entry.
- Documentation: refusal points of the wrapper packages (JSON arrays at about
  5,000 nesting levels, Python parentheses about 3,300, TypeScript calls about
  500, all at the default depth), that a zero-width repetition iteration's
  children are kept, that a synthetic EOF satisfies a trailing grammar `EOF`
  (`p = A B EOF` accepts `[A, B]`), that an EOF token in the middle of the
  list ends the parse (consumers that cannot allow that must reject EOF before
  the last position), and that an empty token list is an error only for
  grammars that cannot match nothing.

### Tests
- Frozen copy of the old engine in `internal/oldengine`, driven by a seeded
  differential test and a fuzz target over 16 adversarial grammars, comparing
  a canonical dump (rule names, positions, nil-ness of children, error text).
- Left-recursion and tree-walker tests under an 8 MB stack cap; nested-group
  frame budget test; hook panic tests; pool reuse after depth, memo-limit,
  syntax-error and panic outcomes; concurrent mixed pass/fail parses; cache
  tests; exact (`reflect.DeepEqual`) reuse comparison; asserted results for
  zero-width repetitions.
- `BenchmarkParseSmallInputLargeGrammar` in `typescript-parser`.
