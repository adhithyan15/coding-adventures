# Changelog

## [0.1.0] - 2026-07-17

### Added

- Initial `j-to-semantic-ir` frontend crate (MA-6e, per
  [HML01](../../../specs/HML01-math-to-semantic-ir.md) §2/§5 and
  [MA06](../../../specs/MA06-j-language.md) §5's own explicit instruction)
  — the last remaining J rollout item, built directly on
  `apl-to-semantic-ir`'s design.
- `compile`/`compile_source` lowering `coding-adventures-j-parser`'s
  `GrammarASTNode` CST into a `semantic_ir::Module`.
- Everything `apl-to-semantic-ir` supports transfers directly: number
  literals (int/float, underscore `_` negative sign instead of APL's
  high-minus `¯`), stranded literals, variables, parenthesised grouping;
  assignment including right-associative chained assignment (`a=.b=.3`,
  J's `=.`/`=:` given identical lowering — not meaningfully distinct in
  this cut); all 12 scalar dyadic atoms shared with APL (`+ - * % <. >. =
  ~: < > <: >:`), unconditionally `ElementwiseOp`; the 6 with a monadic
  meaning mapped onto the exact `"neg"`/`"sign"`/`"recip"`/`"ceil"`/
  `"floor"` builtins `apl-to-semantic-ir` already introduced; `$`/`i.`/`,`
  (shape-reshape, index-generator-index-of, ravel-catenate), monadic and
  dyadic; `/` (reduce) and `\` (scan), monadic-only; auto-print onto the
  shared `"print"` builtin.
- Two genuinely new primitives with no APL analogue: `#` (monadic tally →
  new `"tally"` builtin, dyadic replicate → new `"replicate"` builtin) and
  `^` (monadic exponential → new `"exp"` builtin, dyadic power →
  `Expr::ElementwiseOp { op: Pow, .. }`, reusing the `Pow` variant SIR22
  already has from MATLAB's `.^` but APL's cut never used). Both are
  classified as bespoke non-scalar verbs (matching `j-runtime::eval::JFn`'s
  own categorisation exactly), which correctly excludes both from
  reduce/scan eligibility — keeping this frontend's accepted surface in
  lockstep with the reference interpreter's.
- Trains — `(f g)` hooks, `(f g h)`/`(n g h)` forks, `f@g` compose — the
  one genuinely new production relative to APL (MA06 §3), lowering to
  nested `ElementwiseOp`/`BuiltinCall`/etc. applications with no new SIR
  node at all, per MA06 §5's own instruction. 4+-tooth trains fold
  peel-from-the-left recursively (`(a b c d)` = `(a (b c d))`).
- A dedicated `MAX_TRAIN_COMBINATOR_DEPTH` (12) guard, separate from the
  general expression-depth cap: a hook or verb-left fork duplicates its
  noun operand(s) in the emitted `Expr` tree (this lowerer builds owned
  expression trees, so using an operand twice means cloning an
  already-lowered subtree, unlike a real interpreter which evaluates once
  and reuses the resulting value), and this duplication compounds
  multiplicatively through three mechanisms sharing one counter and cap:
  folding a wide single train, descending into an explicitly nested
  parenthesised sub-train, and — caught by a security review during this
  same PR, after the first two mechanisms were already correctly guarded
  — a *chain* of separately-parenthesised hooks/forks joined by ordinary
  right-recursive application (`(f g)(h i)(j k)...base`). Each `(...)` in
  such a chain is individually within the cap, but the outer application's
  own `.clone()` still duplicates the (already-duplicated) result of the
  rest of the chain, so an earlier draft that reset the combinator-depth
  counter to `0` per application link instead of accumulating it across
  the chain left this specific shape completely unguarded — confirmed to
  blow up to hundreds of megabytes of emitted `Expr` tree from an
  under-100-byte source string before the fix (regression test:
  `a_chain_of_separately_parenthesised_hooks_wider_than_the_cap_is_rejected`).
  Bounds the worst case to `2^12` duplicated copies regardless of which
  mechanism (or mixture) causes the depth. A separate, purely defensive
  `MAX_TRAIN_TEETH` (64) cap bounds a single train's raw tooth count
  before any O(tooth count) collection work.
- Explicit, disclosed rejections (each a clean `JLowerError`): the 6
  comparison atoms used monadically; a reduce/scan-decorated verb used
  dyadically; `$`/`i.`/`,`/`#`/`^` decorated with an adverb (none is a
  scalar dyadic verb); a bare noun tooth anywhere except a fork's leading
  position (`j.grammar`'s own disclosed example, `(A B)`, parses
  syntactically but is semantically invalid); trains/compose nested
  deeper than the combinator-depth cap, or a single train wider than the
  tooth-count cap.
- 47 tests: 42 in `tests/test_lower.rs`, 4 in `tests/test_validator.rs`
  (mirroring `apl-to-semantic-ir`'s own capability-rejection pattern,
  extended to confirm hook/fork-using modules — ordinary nested base-cut
  applications with no new SIR node — are accepted by
  `semantic-ir-to-javascript` exactly like a plain `ElementwiseOp`
  module), 1 doctest.
