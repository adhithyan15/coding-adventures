# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Initial release — **MA-6d** of the J frontend: a tree-walking evaluator
  over `array-runtime` (spec [`MA06`](../../../specs/MA06-j-language.md)).
- `Interpreter` (persistent workspace) + `feed`/`eval` entry points. Auto-print
  semantics: an assignment is silent, a bare `noun_expr` result auto-prints
  (mirrors `apl-runtime`'s own real-session convention).
- Right-to-left evaluation with no precedence cascade, reused directly from
  APL's own grammar shape (MA06 §3) — no precedence climbing anywhere in
  this crate.
- All 12 `BinOp`-mappable primitive verbs (`+ - * % <. >. = ~: < > <: >:`),
  each with its documented monadic meaning where one exists (conjugate,
  negate, sign, reciprocal, floor, ceiling — the six comparisons have no
  monadic form, a clean error) and its dyadic `ops::elementwise` meaning.
  `<.`/`>.` map onto the same `Min`/`Max` `BinOp` variants APL's own
  `⌊`/`⌈` already use — same underlying behavior, different ASCII digraph
  spelling (MA06 §4).
- `$`/`i.`/`,` — bespoke monadic+dyadic primitives (shape/reshape,
  index-generator/index-of, ravel/catenate), re-derived fresh in this
  crate's own `builtins.rs` (the APL originals are private to
  `apl-runtime`) but kept behaviorally consistent across the two
  frontends — **except** `i.`, which is deliberately **0-based**
  (`i.5 == 0 1 2 3 4`), unlike APL's 1-based `⍳5`. Dyadic `i.`'s
  not-found sentinel is `i.`'s own tally (`a.len()`), not APL's
  `len() + 1`.
- `#` (tally/replicate) and `^` (exponential/power) — two genuinely new
  primitives with no APL precedent at all. `^` is implemented entirely
  locally (no new `array_runtime::ops::BinOp` variant, per MA06 §2's
  explicit "no new substrate needed" scope) via a small `elementwise_pow`
  helper mirroring `ops::elementwise`'s exact broadcast-rule structure.
  Dyadic `#` is scoped to a rank ≤ 1 right operand (a documented,
  disclosed simplification, mirroring `array_runtime::ops::outer`'s own
  rank-limiting convention); a rank-2 right operand is a clean error.
- `/` (reduce), `\` (scan) lowered onto `array_runtime::ops::{reduce, scan}`
  — inherently monadic derived verbs; applying one dyadically, or stacking
  an adverb onto `$`/`i.`/`,`/`#`/`^`, is a clean scope error.
- `@` (compose/"atop") — J's one in-scope conjunction. Monadic formula
  (`f (g y)`) is MA06 §4's own; the dyadic formula (`f (x g y)`) is this
  crate's own considered generalization, disclosed in `eval.rs`'s doc
  comments, matching real J's standard atop semantics.
- **Trains — the one genuinely new evaluation shape, with no APL
  precedent.** `JFn` grows `Compose`/`Hook`/`Fork` variants beyond
  `apl-runtime::eval::AplFn`'s shape (per MA06 §5's own explicit
  instruction to generalize it). Hook (`(f g)`) and fork (`(f g h)`,
  including the leading-noun case `(n g h)`) evaluate per MA06 §3's exact
  formulas; a 4+-tooth train folds via `fold_train`'s peel-from-the-left
  recursion (`(a b c d) = Hook(a, Fork(b, c, d))`), following this spec's
  own corrected folding rule (an earlier draft of MA06 §3 described the
  recursion in the opposite, incorrect direction — already fixed in the
  spec before this crate was implemented).
- J-style display (`value.rs`): a leading underscore `_` for negatives
  (never ASCII `-`, which is reserved for the `MINUS` verb token, and
  never APL's high-minus `¯`, which has no ASCII spelling), no trailing
  `.0` on whole-valued floats, no name/`ans =` prefix, space-separated
  vectors, right-aligned matrix rows.
- DoS guards: an independent recursion-depth guard in the evaluator
  (exercised not just by the noun-expression walk, like APL's own guard,
  but also by `apply_monadic`/`apply_dyadic` themselves, since
  `Compose`/`Hook`/`Fork` recurse back through those two functions — a
  genuinely new recursion shape APL's evaluator never needed to guard), and
  a `MAX_ARRAY_LENGTH` (1,000,000) cap on every primitive whose output size
  or work is driven by runtime-computed values — monadic `i.n`, dyadic
  `$`'s target element count, dyadic `,`'s combined output length, dyadic
  `i.`'s `len(a)×len(b)` work, and dyadic `#`'s total replicated output
  length — every one checked *before* allocating or scanning, mirroring
  `apl-runtime`'s own already-security-reviewed guard set (ported forward
  rather than re-discovered from scratch).

### Notes

- A direct white-box unit test of the depth-guard mechanism itself
  (`eval.rs`'s `depth_guard_trips_after_max_depth_and_recovers`) was added
  beyond `apl-runtime`'s own test suite (which has no such test at all) —
  `j-parser`'s `MAX_RULE_DEPTH` (70) bounds any real parsed tree far below
  this evaluator's own `MAX_DEPTH` (512), so a realistic `feed()` call can
  never actually trip the guard; a direct `enter()`/`DepthGuard` test is the
  only way to exercise it at all. Disclosed here since it is a small,
  deliberate addition beyond the literal `apl-runtime` mirror the rest of
  this crate otherwise follows closely.
