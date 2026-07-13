# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Initial release — the SIR23 symbolic-expression + pattern/rewrite runtime
  (HML01 Stream B, item 6), imported by Semantic-IR-emitted TypeScript/
  JavaScript as `__SirSym` for the Wolfram/Macsyma/Maxima CAS domain.
- Re-exports `@coding-adventures/cas-pattern-matching`'s `Bindings`,
  `blank`/`blankTyped`/`named`/`rule`/`ruleDelayed`, `isBlank`/`isPattern`/
  `isRule`, `matchPattern`, `applyRule`, `substitute` unchanged — these
  already faithfully port the Rust `cas-pattern-matching` crate's five-case
  structural matcher algorithm.
- `apply(head, args)` — thin wrapper over `@coding-adventures/symbolic-ir`'s
  `app`, named to match the SIR23 spec's `__SirSym.apply(...)` call-site
  convention.
- **`replaceAll(expr, rules)`** — Wolfram `/.`, one pass: bottom-up walk,
  first-match-wins per subtree, no retry at a position. Genuinely new code
  (not a port — `cas-pattern-matching` has no equivalent of this single-pass
  operator).
- **`replaceRepeated(expr, rules, maxIterations)`** — Wolfram `//.`, a fixed
  point. Algorithmically mirrors `cas-pattern-matching`'s own `rewrite()`
  (bottom-up, per-node local fixed point, global iteration cap returning
  `RewriteCycleError`) but is reimplemented rather than called directly, to
  add the recursion-depth cap below.
- **`MAX_TERM_DEPTH` (512) + `DepthLimitError`/`isDepthLimitError`** — an
  explicit recursion-depth cap on `replaceAll`/`replaceRepeated`'s full-tree
  walk, found necessary during design: `cas-pattern-matching`'s `rewrite()`
  has no depth guard at all, and this package runs on compiled, potentially
  deeply-nested runtime expressions rather than short hand-authored rule
  literals, so carrying that gap forward would reopen a stack-overflow DoS.
  The re-exported matcher primitives (`matchPattern`/`substitute`) don't
  need the same treatment — their recursion depth is bounded by a single
  rule's static pattern/RHS structure, not by the runtime expression being
  matched against; see `src/index.ts`'s module doc comment for the full
  reasoning.
### Fixed

- **`replaceRepeated`'s retry-on-fire step no longer recurses natively** —
  found by this package's own `/security-review` before its first push.
  The initial implementation mirrored `cas-pattern-matching`'s `rewrite()`
  exactly, including a recursive `walk(replacement, depth)` call each time a
  rule fired at a tree position — one more native stack frame per firing,
  bounded only by the caller-supplied `maxIterations`, not by
  `MAX_TERM_DEPTH`. A caller passing a large `maxIterations` on a slowly- or
  never-converging rule set could exhaust the stack through that path
  alone, regardless of how shallow the input expression was — the exact
  class of bug `MAX_TERM_DEPTH` exists to close, reopened via a second,
  unguarded path. Fixed by making the retry a local loop instead of a
  recursive call: firing a rule now just updates `current` and loops back
  (same call frame), so repeated firings at one position cost O(1) native
  stack frames however many times they occur; `depth` (and thus
  `MAX_TERM_DEPTH`) now only increases on a genuine descent into `head`/
  `args`, and `maxIterations` bounds iteration count (CPU time) only, never
  native recursion depth. Verified with a regression test cycling two
  non-deepening rules (`a -> b`, `b -> a`) 50,000 times without a crash —
  the earlier, recursive design would have overflowed the stack at that
  volume.

- `rule`/`ruleDelayed` currently match and substitute identically (no
  general expression evaluator exists yet in this runtime to make the
  eager-vs-delayed RHS-evaluation distinction observable) — documented and
  locked in by a dedicated test, with the `delayed` flag still round-tripping
  faithfully through the data model for future work to extend.
- This PR also files a small, directly-motivating fix in a sibling package:
  `@coding-adventures/symbolic-ir` (this package's core dependency) had no
  `BUILD`/`BUILD_windows` file at all, meaning it wasn't a discovered node
  in the build tool's dependency graph — every existing consumer's edge to
  it (including `cas-pattern-matching`'s) was silently dropped as an
  "external" dependency. Added a minimal `ts_library` `BUILD` file for it.
- And a spec correction: `SIR23-symbolic-pattern-semantic-ir.md` §"Matcher
  semantics" point 1 said backends must walk the tree "top-down,
  left-to-right over `args`" — but `cas-pattern-matching`'s actual
  `rewrite()` (the algorithm this point is supposed to describe) walks
  **bottom-up** (post-order). Corrected the spec text to match the real,
  tested algorithm this package ports, rather than silently implementing
  the opposite of what the (wrong) prose said.
