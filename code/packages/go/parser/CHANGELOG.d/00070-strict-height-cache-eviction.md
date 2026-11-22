## [0.4.2] - 2026-11-16

Follow-ups from the second G2 review round.

### Security
- **The tree-height bound is now strict:** a successful parse returns a tree of
  at most `MaxDepth` levels (root = 1). Each memo slot records the height of
  the subtree it built. A memo hit re-checks `depth + height` against
  `MaxDepth` (a subtree built high in the tree and replayed deeper used to
  reach 4.4x the limit: 4,900 brackets gave height 44,109), and left-recursive
  growth is charged against the seed's real height (a deep seed plus growth
  used to reach about 2x). Heights of abandoned alternatives are not counted.
  Docs now say "at most `MaxDepth`" (the usable height can be one less when a
  speculative attempt touches the limit).
- When the frame budget is what stopped the parse, the error says so and
  prints the frames in use instead of quoting `MaxDepth`.
- The compiled-grammar cache carries a fingerprint (rule count, address of the
  first rule, hash of rule names and the top-level shape of each body); a
  changed rule list is recompiled automatically. Only same-shape edits deep
  inside a rule body need `ForgetGrammar`.

### Changed
- Cache eviction is a generational reset: when 512 grammars are cached, the map
  is cleared and refills, instead of leaving every later grammar uncached
  forever.
- `HardMaxDepth` documented as reachable only for rules of weight 4 or less
  (200,000 / 4); the frame budget undercuts a larger explicit `MaxDepth` for
  deeper rule bodies.
- `WalkAST` snapshots each node's `Children` when it enters it, matching the old
  recursive semantics when a visitor mutates a parent's children mid-walk.

### Tests
- Reproductions of both height overruns (`TestMemoHitAtDeeperDepthIsChecked`,
  `TestGrowthFromDeepSeedIsChecked`), a property test that every successful
  parse has height at most `MaxDepth`, cache eviction and fingerprint tests,
  the frame-budget message, and the walk-snapshot test.
