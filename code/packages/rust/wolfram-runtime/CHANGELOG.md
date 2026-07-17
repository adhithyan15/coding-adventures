# Changelog

All notable changes to `wolfram-runtime` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project uses
[Semantic Versioning](https://semver.org/).

## [0.19.4] — 2026-07-16

### Added (W-22 — `D`, fourth `cas-*` head)

- `D[expr, x]` — the symbolic derivative of `expr` with respect to the
  symbol `x`: sum/product/quotient/power/chain rules plus the elementary
  transcendental functions (`Sin`, `Cos`, `Exp`, `Log`, `Sqrt`, inverse and
  hyperbolic trig), fully recursed and re-evaluated so constant folding and
  nested rule output collapse into one final form (`D[x^3, x]` → `3 * x^2`,
  not a half-reduced intermediate). Like `Factor`, the actual differentiation
  logic lives directly in `symbolic-vm` itself: this wiring calls the new
  `symbolic_vm::handlers::differentiate` — the exact pipeline Macsyma's own
  `D` already runs, extracted into a `pub` free function specifically for
  this reuse (see `symbolic-vm`'s own changelog). No algorithm is
  reimplemented or duplicated; a parity test pins both languages' call sites
  to agree on the same input, exactly like `Simplify`/`Expand`/`Factor`'s own
  parity tests.
- Like every W-5+ built-in, `D` is an ordinary eager `Head[args]` form
  requiring exactly two arguments, the second of which must be a bare
  symbol; any other shape leaves the form unevaluated. Unlike `Factor`
  (whose arity check lives inside `factor_handler` itself), this wrapper
  does its own check — `derivative_handler`'s existing arity contract
  panics on the wrong count, which is right for `symbolic-vm`'s own internal
  dispatch but wrong for Wolfram's fail-soft contract, so `d_handler`
  validates the shape before ever calling through.

## [0.19.3] — 2026-07-16

### Changed

- `Expand[...]` now collects like terms — `Expand[(x+1)^2]` returns
  `1 + 2*x + x^2`, not the raw `1 + x + x + x*x` from before. No code
  change in this crate: `expand_handler` delegates to
  `cas_simplify::expand` unchanged, which gained a `collect_terms` pass
  (see that crate's 0.5.0 CHANGELOG entry). Updated this crate's own
  `expand_distributes_products_over_sums` test and the `expand_handler`
  doc comment, which had pinned/described the old uncollected shape.

## [0.19.2] — 2026-07-12

### Added (W-22 — `Factor`, third `cas-*` head)

- `Factor[expr]` — factors a univariate integer polynomial, or recognises
  one of a handful of common multivariate patterns (perfect square/cube,
  difference of squares, cubic identities, a common symbolic/integer term
  to pull out, bivariate/n-variate Hensel lifting) — unevaluated if none
  apply. Unlike `Simplify`/`Expand` (thin calls into the standalone
  `cas-simplify` crate), `Factor`'s implementation lives directly in
  `symbolic-vm` itself: this wiring calls
  `symbolic_vm::handlers::factor_handler` directly — the exact function
  Macsyma's own `factor` surface function already calls — made `pub`
  specifically for this reuse (see `symbolic-vm`'s own changelog). No
  algorithm is reimplemented or duplicated; a parity test pins both
  languages' call sites to agree on the same input, exactly like
  `Simplify`/`Expand`'s own parity tests.
- Like every W-5+ built-in, `Factor` is an ordinary eager `Head[args]` form
  requiring exactly one argument; any other arity leaves the form
  unevaluated. That arity check lives inside `factor_handler` itself
  (unlike `simplify_handler`/`expand_handler`, which must unwrap a single
  expression argument themselves before calling a function that only takes
  one bare expression), so this wiring needs no arity check of its own.
- 5 new tests: basic univariate factoring, an unrecognised multivariate
  form staying unevaluated, the Wolfram/Macsyma parity check, wrong-arity
  fail-soft, and full parser→lower→backend dispatch.
- Marks `Factor` delivered in `MA04-wolfram-language.md` §24.

## [0.19.1] — 2026-07-11

### Fixed (W-13 — quadratic set-op DoS)

- `Union`/`Intersection`/`Complement`/`DeleteDuplicates`/`Tally` (W-13) each
  called `contains_element` — an O(n) linear membership scan — once per
  input element against a growing accumulator, making every one of these
  heads worst-case O(n²) despite the existing `MAX_LIST_LENGTH` (1,000,000)
  cap: a single input of ~1,000,000 genuinely distinct elements took
  30-40+ minutes and 100-200% CPU to reach the cap (confirmed by direct
  measurement — `union_over_cap_stays_unevaluated`/
  `tally_over_cap_stays_unevaluated`, this crate's own pre-existing tests,
  were the accidental discovery). `IRNode` carries an `f64` and so isn't
  `Hash`-keyable, but it *is* totally ordered (`canonical_cmp`), so every
  head now sorts once (O(n log n)) instead of scanning repeatedly:
  `sorted_dedup` for `Union`, sorted two-pointer merges
  (`sorted_intersect`/`sorted_difference`) for `Intersection`/`Complement`,
  and a single grouping pass (`group_by_first_occurrence`) for the two
  order-preserving heads, `DeleteDuplicates`/`Tally`. `MemberQ` is
  unchanged — a single membership query was never the quadratic-blowup
  source (that shape only exists when the same O(n) check runs once per
  element of a *growing* accumulator).
- Net effect: the entire `wolfram-runtime` test suite (332 tests,
  including three new large-distinct-input regression tests for
  `Intersection`/`Complement`/`DeleteDuplicates` alongside the pre-existing
  `Union`/`Tally` ones) now runs in well under a second, down from
  30-40+ minutes for the two slowest tests alone.
- No observable output change for any input under the cap — every
  existing correctness test (sort order, first-occurrence order, numeric
  subtype distinctness, `NaN`-safety via `canonical_cmp`) passes unchanged.

## [0.19.0] — 2026-07-05

The **W-22** deliverable (MA04 §2), second head: `Expand`. The blocker
noted in 0.18.0's release ("Remaining: `Expand`, `Factor`, ...") is now
cleared — a prior fix (`macsyma-runtime`, "add `cas_simplify::expand`,
wire it as Macsyma's `Expand` handler") gave `cas-simplify` a real
`expand()` function; this release wires it under the Wolfram head name the
same thin way `Simplify` was wired.

### Added (W-22 — `Expand`)

- `Expand[expr]` — a thin call into `cas-simplify`'s existing `expand()`
  (distributes `Mul` over `Add`/`Sub`, expands bounded non-negative integer
  `Pow` via square-and-multiply), the exact function Macsyma's own
  `expand()` surface function calls, including its internal
  `EXPAND_MAX_POW`/`EXPAND_MAX_TERMS` DoS guards. No new algorithm or guard
  — Wolfram and Macsyma now agree on every expansion this crate can
  perform, by construction (a shared test pins the two call sites to the
  same result on the same input, mirroring `Simplify`'s parity test).
- Honest scope note (inherited from `cas-simplify`, not new to this
  release): `Expand` does not collect like terms —
  `Expand[(x+1)^2]` produces `1 + x + x + x*x`, not `1 + 2*x + x^2`.

### Tests

- 4 new tests: product-over-sum distribution, Wolfram/Macsyma call-site
  parity, wrong-arity fail-soft (unevaluated), and one end-to-end test
  through the full parser → lower → `WolframBackend` path — mirroring the
  four `Simplify` tests added in 0.18.0.

## [0.18.0] — 2026-07-03

The **W-22** deliverable (MA04 §2): the first head of the previously
unnumbered "Future" item — "the `cas-*` function surface under Wolfram names
(`Expand`, `Factor`, `Solve`, `D`, `Integrate`, …) wired to the existing
`cas-*` crates." W-22 lands one head at a time; this release ships the first.

### Added (W-22 — `Simplify`)

- `Simplify[expr]` — a thin call into `cas-simplify`'s existing `simplify()`
  (canonical ordering, constant folding, identity rules, fixed-pointed up to
  50 iterations), the exact function Macsyma's own `simplify()` surface
  function calls. No new algorithm — Wolfram and Macsyma now agree on every
  simplification this crate can perform, by construction (a shared test pins
  the two call sites to the same result on the same input).
- New dependency: `cas-simplify` (0.3.0).

### Tests

- 5 new tests: additive/multiplicative identity folding, constant folding,
  Wolfram/Macsyma call-site parity, wrong-arity fail-soft (unevaluated), and
  one end-to-end test through the full parser → lower → `WolframBackend` path.

## [0.17.0] — 2026-06-25

The **W-21** deliverable (MA04 §23): **lowering** for the W-20 pattern
constructs' operator sugar. W-21 is grammar + lowering only — the four runtime
heads (`Alternatives`, `Condition`, `PatternTest`, `ReplaceRepeated`) shipped and
evaluated in W-20 (§22) and are **reused unchanged**; no new evaluation logic was
added. This crate gains only the lowering that maps the new parser rules to those
heads, so an operator form and its `Head[args]` long form produce identical IR.

### Added (W-21 — operator-sugar lowering)

- `lower_alternatives` — `a | b | c` → one n-ary `Alternatives[a, b, c]` (folded
  flat like `+`/`&&`; a lone operand passes through).
- `lower_condition` — `patt /; test` → `Condition[patt, test]`. Unlike
  `lower_rule`, it deliberately keeps **bare** named-symbol references in the
  test, because the W-20 `Condition` handler substitutes the match's named
  bindings into the test before evaluating it.
- `lower_patterntest` — `patt ? fn` → `PatternTest[patt, fn]` (left-associative
  chain).
- `lower_replaceall` extended — `expr //. rules` → `ReplaceRepeated[expr, rules]`
  beside the existing `/.` → `ReplaceAll`, both at the same left-associative
  level (it now walks operator tokens to pick the head per step). `//.` inherits
  W-20's hard iteration cap (`REPLACE_REPEATED_MAX_ITERATIONS`) and growth cap
  (`REPLACE_GROWTH_NODE_CAP`) verbatim — lowering to the operator is identical to
  writing `ReplaceRepeated[…]`.

### Tests

- 5 lowering unit tests (each operator + `|`-tighter-than-`/;` precedence +
  `?`-chain fold + mixed `/.`/`//.` chain) and 6 end-to-end integration tests
  (the §23 acceptance examples, each asserting operator form == W-20 long form).

## [0.16.0] — 2026-06-22

The **W-20** deliverable (MA04 §22): Wolfram's **advanced pattern constructs**,
built on W-18's matcher, W-19's `replace_all_once`, and the existing
`cas-pattern-matching` crate. **No grammar change** — the four constructs ship as
ordinary head applications (`Alternatives[…]`, `Condition[…]`, `PatternTest[…]`,
`ReplaceRepeated[…]`), which the parser already accepts as `NAME[args]`. The
surface operator sugar (`|`, `/;`, `?`, `//.`) needs new lexer tokens + parser
precedence + lowering + a regenerated grammar, so it is **deferred to W-21** (see
below). W-20 is entirely in `wolfram-runtime`.

### Added (advanced pattern constructs)

- **`Alternatives[a, b, …]`** (`a | b | c`) — matches the subject against each
  alternative **in order**; the first that matches wins (with its bindings). An
  empty `Alternatives[]` matches nothing. Branches nest freely
  (`_Integer | _String`). `MatchQ[2, Alternatives[1, 2, 3]]` → `True`;
  `MatchQ[5, Alternatives[1, 2, 3]]` → `False`. Bounded — each branch is tried
  once, left to right; no cross-branch combinatorial expansion.
- **`Condition[patt, test]`** (`patt /; test`) — matches `patt`, substitutes the
  captured **named bindings** into `test` (bare-symbol substitution — a test
  references its captures as ordinary symbols, e.g. `x > 2`, not `Pattern[…]`
  nodes), and accepts **only if** `test` evaluates to `True`.
  `Cases[{1,2,3,4}, Condition[Pattern[x, Blank[]], x > 2]]` → `{3, 4}`. The test
  runs through a **fresh, stateless** `WolframBackend`-backed VM (it is pure and
  must not see/mutate session state) and via the standard bounded evaluator.
- **`PatternTest[patt, fn]`** (`patt ? fn`) — matches `patt`, then accepts only if
  `fn[subject]` evaluates to `True` (the *subject*, not a binding).
  `MatchQ[4, PatternTest[Blank[], EvenQ]]` → `True`;
  `MatchQ[3, PatternTest[Blank[], EvenQ]]` → `False` (the W-9 `EvenQ` predicate).
- **`ReplaceRepeated[expr, rules]`** (`expr //. rules`) — apply `ReplaceAll`
  repeatedly to a **fixed point**, evaluating between passes, until either a pass
  produces a result identical to its input (convergence) or the pass count reaches
  `REPLACE_REPEATED_MAX_ITERATIONS` (`2^16`, the same order of magnitude as
  Wolfram's default `MaxIterations`). `ReplaceRepeated[{1,2,3}, Rule[2, 99]]` →
  `{1, 99, 3}` (converges); `ReplaceRepeated[{1,2}, {Rule[1,2], Rule[2,3]}]` →
  `{3, 3}` (genuinely iterates). Held (joins `PATTERN_HEADS`), so its rules survive
  literally; only the subject is evaluated up front.

### Safety

- **Hard iteration cap on `ReplaceRepeated`.** A self-recursive rule like
  `ReplaceRepeated[x, x -> f[x]]` never converges — the term changes every pass,
  so the equality check never fires. The iteration counter is what terminates the
  loop: at the cap it returns the **last form** with **no hang, no panic, and no
  unbounded memory**. Each individual pass is still depth-guarded by
  `REPLACE_MAX_DEPTH`, so both the inner (tree depth) and outer (pass count) loops
  are bounded. A dedicated test asserts the self-recursive case returns rather
  than hanging.
- **Bounded backtracking.** `Alternatives` tries each branch once, left to right —
  no exponential cross-product. The advanced-construct dispatch recurses through
  the same depth discipline as W-18/W-19.
- **Bounded test evaluation.** `Condition`/`PatternTest` evaluate their tests
  through the standard VM with its existing recursion/stack guards; the fresh VM
  has no session state to corrupt. Anything other than `True` fails the match.
- **No panic on malformed nodes.** The `pattern_tree_well_formed` guard still
  rejects malformed `Pattern[…]` before any indexing; a malformed
  `Alternatives`/`Condition`/`PatternTest` (wrong arity) simply **fails to match**,
  and `ReplaceRepeated` with the wrong arity stays unevaluated.

### Deferred to W-21

The operator **sugar** for all four constructs (`|`, `/;`, `?`, `//.` — needs a
grammar change), the **sequence patterns** `__` (`BlankSequence`) / `___`
(`BlankNullSequence`) — variable-arity matching is not yet in the shared
`cas-pattern-matching` matcher, so the `ReplaceRepeated` tests use non-sequence
rules — plus **`Repeated`** `patt..`, **`Except`**, **`Longest`/`Shortest`**, and
**`Replace` level specifications** (the third argument).

## [0.15.0] — 2026-06-21

The **W-19** deliverable (MA04 §21): Wolfram's **named patterns** and
**replacement rules**, built directly on W-18's matcher and the existing
`cas-pattern-matching` crate. **No grammar change** — `->`/`:>`/`/.`/`_` were
already tokenised and lowered (`x_` → `Pattern[x, Blank[]]`, `a -> b` →
`Rule[a, b]`, `a :> b` → `RuleDelayed[a, b]`, `e /. r` → `ReplaceAll[e, r]`).
W-19 is entirely in `wolfram-runtime`: it upgrades the matcher to bind named
captures, adds the `Replace` handler, and replaces the `/.` pre-pass's rewrite
engine with a correct single top-down pass.

### Added (named patterns & replacement)

- **Named-pattern binding.** `pattern_matches` now delegates to
  `cas_pattern_matching::match_pattern`, the shared matcher that records
  `name → subexpr` captures. So a named blank `x_` (`Pattern[x, Blank[]]`) and a
  typed named blank `x_Integer` (`Pattern[x, Blank[Integer]]`) match **and bind**.
  `MatchQ`, `Cases`, and `FreeQ` are upgraded in place: `MatchQ[2, x_]` → `True`
  (was `False` under the binding-free W-18 matcher), `Cases[{1, 2, 3}, x_Integer]`
  → `{1, 2, 3}`. All W-18 results (literals, `_`, `_h`) are preserved — the new
  matcher is a strict superset, and `IRNode`'s `PartialEq` keeps `2 ≠ 2.0` exactly
  as `same_element` did.
- **`ReplaceAll[expr, rules]` / `expr /. rules`** — a single **top-down
  leftmost-outermost** pass: at each node the rules are tried in order; the first
  whose LHS matches the **whole** node wins, its RHS (with captures substituted)
  replaces the node, and the pass does **not** re-descend into the result. No
  match → recurse into the head and arguments. `f[2] /. f[x_] -> x` → `2`;
  `g[1, 2] /. g[a_, b_] -> a + b` → `3`; `{1, 2, 3} /. 2 -> 99` → `{1, 99, 3}`;
  `ReplaceAll[{1, 2, 3}, x_Integer -> x^2]` → `{1, 4, 9}`. `rules` may be a single
  rule or a `List` of rules (first match per node wins).
- **`Replace[expr, rules]`** — like `ReplaceAll` but matches **only the whole
  `expr`** (no descent into parts). `Replace[5, x_ -> x + 1]` → `6`;
  `Replace[{1, 2, 3}, x_Integer -> 0]` → `{1, 2, 3}` (the list's head is `List`,
  not `Integer`, and `Replace` does not descend). Held (joins `PATTERN_HEADS`);
  evaluates only its subject; a non-two-argument call — including the deferred
  three-argument *level-spec* form — stays unevaluated.
- **`Rule` (`->`) / `RuleDelayed` (`:>`)** — both carry a pattern LHS and a
  template RHS. Because replacement runs as an IR-level pre-pass *before*
  evaluation, the RHS of both is held until its captures are substituted, then
  evaluated once; `h[3] /. h[n_] :> n + 1` → `4`.

### Fixed

- **`ReplaceAll` no longer loops to non-convergence.** The prior `/.` pre-pass
  used `cas_pattern_matching::rewrite`, a **bottom-up fixed-point** rewriter that
  re-walks each replacement; `{1, 2, 3} /. x_Integer -> x^2` looped forever
  (`1` → `1^2` → folds to `1`, an `Integer`, re-matching `x_Integer` …) and
  errored "did not converge". The W-19 single-pass `replace_all_once` visits each
  node at most once, yielding `{1, 4, 9}` and stopping.

### Safety

- **Bounded recursion.** `replace_all_once` is depth-guarded by `REPLACE_MAX_DEPTH`
  (512, mirroring `FREEQ_MAX_DEPTH`): past the cap a sub-node is returned unchanged
  rather than recursed, turning a crafted pathologically nested input into a safe
  bounded answer instead of a stack overflow. The whole pre-pass still runs inside
  the bounded-stack `catch_unwind` worker thread.
- **No loops, no panics.** A single non-re-descending pass cannot expand
  unboundedly (the old `MAX_REWRITE_ITERATIONS` cap is gone with the fixed-point
  rewriter). An unbound RHS capture is left in place by `substitute` (no panic); a
  non-rule operand yields an empty rule set, returning the subject unchanged;
  heterogeneous compares go through total `PartialEq`.

### Deferred to W-20

Alternatives (`a | b`, `Alternatives`), conditions (`patt /; test`, `Condition`),
`PatternTest` (`patt ? fn`), sequences (`__`/`___`, `BlankSequence`/
`BlankNullSequence`), `Repeated`, **level specifications** for `Replace`, and
**`ReplaceRepeated`** (`//.`, the fixed-point form). See MA04 §21.7.

## [0.14.0] — 2026-06-21

The **W-18** deliverable (MA04 §19): Wolfram's **pattern-matching predicates**
`MatchQ`, `Cases`, and `FreeQ`, lowered onto the same substrate as the rest of
the lane. All ordinary `Head[args]` forms — **no grammar change**; only the
`wolfram-runtime` builtin handler table grows. The three heads are **held** (a
new `PATTERN_HEADS` set folded into the `WolframBackend` hold set alongside the
W-7/W-8/W-14 held heads) so the **pattern** argument arrives **literal** — a
pattern is a *form*, not a value, exactly as `Switch` relies on. Each handler
evaluates **only its subject**.

### Added (pattern-matching predicates)

- **`MatchQ[expr, patt]`** — `True` iff `expr` matches `patt`, else `False`.
  `MatchQ[2, _]` → `True`, `MatchQ[2, _Integer]` → `True`, `MatchQ[2, 2]` →
  `True`, `MatchQ[2, 3]` → `False`. The subject is evaluated; the pattern stays
  literal. Wrong arity is left **unevaluated**.
- **`Cases[list, patt]`** — the `List` of `list`'s elements that match `patt`,
  dropping non-matches. `Cases[{1, 2, 3, 4}, _]` → `{1, 2, 3, 4}`,
  `Cases[{1, 2, 3}, 2]` → `{2}`, `Cases[{1, 2.0, 3}, _Integer]` → `{1, 3}`. A
  **non-list** first argument (or wrong arity) is left unevaluated. The result
  inherits the input's `MAX_LIST_LENGTH` bound.
- **`FreeQ[expr, form]`** — `True` iff `form` occurs **nowhere** within `expr`
  (recursively — the root, every `Apply` head, and every argument), else
  `False`. `FreeQ[{1, 2, 3}, 2]` → `False`, `FreeQ[{1, 2, 3}, 5]` → `True`,
  `FreeQ[f[g[2]], g]` → `False`, `FreeQ[f[g[2]], h]` → `True`.

### Pattern subset and matcher

The supported pattern vocabulary is a literal (structural equality via the W-13
`same_element` comparator), `_` (`Blank[]`, the catch-all), and a head-typed
`_h` (`Blank[h]`, matching iff the subject's Wolfram head is `h`). A single
panic-free `pattern_matches` primitive extends the W-14 `Switch` matcher by
**enforcing** the `Blank[h]` head constraint that `Switch` ignored — the one
capability W-18 needed beyond W-14. The lowerer turns `_Integer` →
`Blank[Integer]`, `_Real` → `Blank[Real]`, `_Symbol` → `Blank[Symbol]`, and the
head map sends an `Integer` atom → head `Integer`, a `Float` atom → head `Real`,
a symbol → head `Symbol`; so `MatchQ[2.0, _Integer]` is `False` (a float is
`_Real`).

### Robustness

- **`FreeQ` recursion is depth-bounded** (`FREEQ_MAX_DEPTH = 512`): the tree is
  already size-bounded by the parser's nesting cap and `MAX_LIST_LENGTH`, and at
  the cap the walk stops descending and reports "occurs" conservatively, so a
  crafted over-deep input yields a **safe bounded answer instead of a stack
  overflow** — never a panic.
- **Heterogeneous atom comparison is total** — comparing across `Integer` /
  `Float` / `Symbol` / `String` / `Rational` kinds simply reports no match and
  never panics.

### Deferred to W-19 (MA04 §19.6)

The richer pattern algebra is **explicitly out of scope** for W-18: named
patterns `x_` (`Pattern[x, Blank[]]`, capture binding), alternatives `a | b`
(`Alternatives`), conditions `patt /; t`, `PatternTest`, blank sequences `__`
(`BlankSequence`), and replacement `/.` / `Replace`. A named blank
`Pattern[x, Blank[…]]` falls through to the literal branch and simply fails to
match an evaluated value (rather than mis-binding) — the safe documented W-18
behaviour until W-19 adds capture binding.

### Tests

- 9 handler-level unit tests in `wolfram-runtime` (literal / blank / head-typed,
  the Integer-vs-Real head distinction, `Cases` filtering plus empty / non-list,
  `FreeQ` membership / nesting plus the depth-bound no-overflow guard and
  heterogeneous no-panic case, wrong-arity-unevaluated for `MatchQ`/`FreeQ`).
- 1 end-to-end `wolfram-repl` test exercising the real `_Integer` / `_Real`
  lowering through parse → lower → eval.

## [0.13.0] — 2026-06-21

The **W-16** deliverable (MA04 §19): Wolfram's **nested/structured list
operations** — the *shape* vocabulary for matrix-like nested lists, layered on
top of the W-9 flat-list heads. All ordinary eager `Head[args]` forms — **no
grammar change**; only the `wolfram-runtime` builtin handler table grows. Every
head reuses the W-9 list machinery (`list_elements`, `apply(sym(LIST), …)`, the
`MAX_LIST_LENGTH` cap). `Flatten` already existed (W-9) and is **not**
reimplemented.

### Added (nested/structured list operations)

- **`Transpose[m]`** — transpose a rectangular list of lists (rows ↔ columns).
  `Transpose[{{1,2},{3,4}}]` → `{{1,3},{2,4}}`. A ragged matrix, a list of
  non-lists, an empty list, or a non-list argument is left **unevaluated**. The
  output element count equals the input's — no new DoS surface.
- **`Dimensions[expr]`** — the dimensions of the largest rectangular nested
  array, as a list. `Dimensions[{{1,2,3},{4,5,6}}]` → `{2,3}`; a scalar → `{}`;
  ragged nesting reports only the rectangular prefix (`Dimensions[{{1,2},{3}}]`
  → `{2}`).
- **`Partition[list, n]` / `Partition[list, n, d]`** — consecutive length-`n`
  sublists stepping by `d` (default `d = n`). `Partition[{1,2,3,4},2]` →
  `{{1,2},{3,4}}`; `Partition[{1,2,3,4,5},2,1]` → `{{1,2},{2,3},{3,4},{4,5}}`. A
  trailing partial block is **dropped** (Wolfram default — no padding). `n`/`d`
  must be positive integers. **Output-capped**: the block count and total
  element count (`blocks × n`, via `checked_mul`) are checked against
  `MAX_LIST_LENGTH` before allocating.
- **`Take[list, n]` / `Take[list, -n]`** — first `n` / last `n` elements. The
  **list** `Take` (distinct from W-12's `StringTake`). `Take[{1,2,3,4,5},2]` →
  `{1,2}`; `Take[{1,2,3,4,5},-2]` → `{4,5}`. An out-of-range/non-integer count or
  non-list argument is left unevaluated; the count is range-checked in `i128` so
  a crafted `i64::MIN` cannot overflow.
- **`Drop[list, n]` / `Drop[list, -n]`** — drop first `n` / last `n` elements.
  The **list** `Drop` (distinct from W-12's `StringDrop`). `Drop[{1,2,3},1]` →
  `{2,3}`; `Drop[{1,2,3},-1]` → `{1,2}`. Same validation/no-overflow contract as
  `Take`.
- **`ConstantArray[c, n]` / `ConstantArray[c, {m, n}]`** — a length-`n` list, or
  an `m`×`n` nested list, of copies of `c`. `ConstantArray[0,3]` → `{0,0,0}`;
  `ConstantArray[5,{2,2}]` → `{{5,5},{5,5}}`. **The primary W-16 DoS surface**:
  the total element count is guarded *before* any allocation — 1-D `n` is capped
  at `MAX_LIST_LENGTH`; 2-D `m × n` is computed with **`checked_mul` on i128**
  and both `m` and `m × n` are capped, so a tiny spec like
  `ConstantArray[0,{10^6,10^6}]` is refused (unevaluated) rather than allocated.

### Notes

- Take/Drop are the **list** heads; W-12's `StringTake`/`StringDrop` keep
  operating on strings — the two families never collide.
- Spec: MA04 §19 (and the §2 pieces list) documents the new builtins, the
  Partition step/partial-drop rule, and the ConstantArray output-cap behaviour.

## [0.12.0] — 2026-06-20

The **W-15** deliverable (MA04 §18): Wolfram's **numeric & integer math**
functions, lowered onto the same substrate as the rest of the lane. All ordinary
eager `Head[args]` forms — **no grammar change**; only the `wolfram-runtime`
builtin handler table grows. Integer ops stay **exact** (i64, computed in i128
with overflow guards); real ops use f64, mirroring the IR's own
`Integer`/`Float` split. `Mod`, `Power`, and `N` already existed and are **not**
duplicated. `Sqrt` is overridden in the Wolfram table (which precedes the inner
`SymbolicBackend` in `handler_for`) to give Wolfram-exact semantics.

### Added (numeric & integer math)

- **`Abs[x]`** — absolute value; exact for integers, f64 for reals.
  `Abs[-3]` → `3`, `Abs[-2.5]` → `2.5`. `Abs[i64::MIN]` (magnitude one past
  `i64::MAX`) is left **unevaluated** rather than overflowing.
- **`Sign[x]`** — `−1` / `0` / `1` by sign; always an exact integer.
  `Sign[-2]` → `-1`, `Sign[0]` → `0`. Signed zero is zero; `Sign[NaN]` is left
  unevaluated.
- **`Min[a, b, …]` / `Max[a, b, …]`** — also over a single list `Min[{…}]`. The
  original node (exact integer where applicable) is returned: `Min[3, 1, 2]` →
  `1`, `Max[{3, 1, 2}]` → `3`. A non-numeric operand or empty fold is left
  unevaluated.
- **`Floor[x]` / `Ceiling[x]` / `Round[x]`** — always an integer result.
  `Floor[2.7]` → `2`, `Floor[-2.1]` → `-3`, `Ceiling[2.1]` → `3`. **`Round` is
  half-to-even** (banker's rounding), matching Wolfram: `Round[2.5]` → `2`,
  `Round[3.5]` → `4` (Rust's `f64::round` rounds half away from zero, so it is
  not used). `f64 → i64` conversion saturates; non-finite inputs are left
  unevaluated.
- **`Quotient[m, n]`** — integer division toward −∞ (floor division).
  `Quotient[7, 2]` → `3`, `Quotient[-7, 2]` → `-4`. Computed in i128 so
  `Quotient[i64::MIN, -1]` is left unevaluated rather than panicking;
  `Quotient[m, 0]` is undefined → unevaluated.
- **`GCD[a, b, …]` / `LCM[a, b, …]`** — non-negative, integer-only. `GCD[12, 18]`
  → `6`, `GCD[12, 18, 24]` → `6`, `LCM[4, 6]` → `12`, `LCM[…, 0]` → `0`. Folded
  in **i128**; `LCM` divides by the gcd first (`a / g * b`, never `a * b / g`)
  and range-checks the result — an over-i64 LCM (e.g. of two large coprime ints)
  is left **unevaluated**, never wrapped or panicked.
- **`Sqrt[x]`** — exact integer for perfect squares (`Sqrt[16]` → `4`,
  `Sqrt[0]` → `0`); a non-perfect-square non-negative integer is left **symbolic**
  (`Sqrt[2]` → `Sqrt[2]`, with the float available via `N[Sqrt[2]]`). A `Float`
  argument numericises (`Sqrt[2.0]` → `1.4142…`); a negative argument is left
  unevaluated (no complex numbers). The perfect-square test squares the candidate
  root in i128 so it cannot overflow.

### Security / robustness

- All exact-integer ops compute with **i128 intermediates and explicit overflow
  guards** that fail **closed** (echo the application unevaluated) rather than
  wrapping or panicking — `Abs[i64::MIN]`, `Quotient[i64::MIN, -1]`, `GCD` of
  `i64::MIN`, and `LCM` of large coprime integers are all covered by tests.
- Every handler follows the W-5/W-9/W-12/W-13/W-14 fail-soft contract: wrong
  arity, a non-numeric (or non-integer where required) argument, division by
  zero, or a tripped overflow guard leaves the form unevaluated. No panics.

## [0.11.0] — 2026-06-20

The **W-14** deliverable (MA04 §17): Wolfram's **conditionals** and **type
predicates**, lowered onto the same substrate as the rest of the lane. `Which` and
`Switch` join the `WolframBackend` held set (alongside `If`, the W-7 iteration
heads, and the W-8 scoping heads) so that **only the selected branch is ever
evaluated** — a non-taken branch (which might error or have a side effect) never
runs. `Switch`'s form matching reuses the W-13 `same_element` comparator, so it
agrees with `MemberQ`/`Union` on what "the same" means, and recognises `Blank[]`
(the lowering of `_`) as the catch-all default. The eager `Boole` and the
`NumberQ`/`IntegerQ`/`StringQ`/`ListQ`/`TrueQ` predicates are thin matches over the
`IRNode` kind. Like every head since W-5 these are plain `Head[args]` applications,
so there is **no grammar change**; only the `wolfram-runtime` builtin handler table
and the held set grow. `EvenQ`/`OddQ` (W-9) are left unchanged.

### Added (conditionals — held)

- **`Which[c1, v1, c2, v2, …]`** — evaluate conditions left to right; return the
  value paired with the **first** condition that reduces to `True`. Held: only the
  selected value is evaluated. No true condition → `Null` (the evaluated answer);
  an **odd** argument count (dangling final condition) → left unevaluated.
  (`Which[False, 1, True, 2]` → `2`; `Which[False, 1]` → `Null`;
  `Which[2 > 1, "a"]` → `"a"`.)
- **`Switch[expr, form1, v1, …, _, default]`** — evaluate `expr` once, then match
  it against each **literal** `formi` by structural equality (W-13 `same_element`);
  `Blank[]` (`_`) matches anything as the default. Held: only the selected value is
  evaluated. No match → left unevaluated; an **even** argument count (final
  unpaired form, or missing `expr`) → left unevaluated.
  (`Switch[2, 1, "a", 2, "b", _, "z"]` → `"b"`; `Switch[5, 1, "a", _, "z"]` → `"z"`.)

### Added (conditionals — eager) and type predicates

- **`Boole[cond]`** — `True` → `1`, `False` → `0`; any other (non-boolean)
  argument is left unevaluated. (`Boole[2 > 1]` → `1`; `Boole[1 > 2]` → `0`.)
- **`NumberQ[x]`** — `True` for a real number (`Integer`/`Rational`/`Float`).
- **`IntegerQ[x]`** — `True` only for an exact integer (`IntegerQ[2.0]` is `False`).
- **`StringQ[x]`** — `True` for a string literal.
- **`ListQ[x]`** — `True` for a `List[…]` (reuses `is_list`).
- **`TrueQ[x]`** — `True` only for the literal `True` symbol; total — `False` for
  everything else (including a free symbol), never unevaluated.

### Security / robustness

- `Which`/`Switch` evaluate **exactly one** branch via a single `vm.eval`, so a
  non-selected branch cannot double-evaluate, error, or produce a side effect.
- Odd-arity `Which` and even-arity `Switch` (malformed pair lists) are detected
  *before* any `chunks_exact(2)` walk and left unevaluated — no index can run past
  the end of the argument list, no panic. The predicates and `Boole` reject
  arity ≠ 1 the same way. No new unbounded-recursion or growth surface is added
  beyond the single selected-branch `vm.eval`, which the W-4 fuel machinery bounds.

## [0.10.0] — 2026-06-19

The **W-13** deliverable (MA04 §16): Wolfram's **list set / multiset operations**,
lowered onto the *same* substrate as the rest of the lane — the W-9 list machinery
(`list_elements`, `apply(sym(LIST), …)`, the `MAX_LIST_LENGTH` cap) and the W-9
canonical-order comparator `canonical_cmp`, reused both to *sort* the unique
outputs of `Union`/`Intersection`/`Complement` and to define **element-equality**
(two nodes are the same element iff `canonical_cmp` ranks them `Equal`). Like every
head since W-5 these are plain `Head[args]` applications, so there is **no grammar
change**; only the `wolfram-runtime` builtin handler table grows. `Count` (W-9,
predicate form) is left as-is.

### Added (list set operations)

- **`Union[a, b, …]`** — the **sorted**, duplicate-free union of the element lists
  (`Union[{1, 2}, {2, 3}]` → `{1, 2, 3}`; `Union[{3, 1, 2, 1}]` → `{1, 2, 3}`, so a
  single argument doubles as sort-and-unique). DoS-capped at `MAX_LIST_LENGTH` —
  the deduped accumulator is refused (form left unevaluated) before it can exceed
  the cap, symmetric with `Join`/`Flatten`.
- **`Intersection[a, b, …]`** — the **sorted** elements present in *every* argument
  list (`Intersection[{1, 2, 3}, {2, 3, 4}]` → `{2, 3}`).
- **`Complement[all, x, …]`** — the **sorted** elements of `all` not in any of
  `x, …` (`Complement[{1, 2, 3, 4}, {2, 4}]` → `{1, 3}`).
- **`DeleteDuplicates[list]`** — first-occurrence-order dedup, **order-preserving**
  and deliberately *not* sorted (`DeleteDuplicates[{3, 1, 1, 2, 3}]` → `{3, 1, 2}`,
  contrast with `Union`'s `{1, 2, 3}`).
- **`MemberQ[list, elem]`** — `True`/`False` whether `elem` is an element of
  `list` (`MemberQ[{1, 2, 3}, 2]` → `True`; `MemberQ[{1, 2, 3}, 9]` → `False`).
- **`Tally[list]`** — `{element, count}` pairs in first-occurrence order
  (`Tally[{a, a, b, a}]` → `{{a, 3}, {b, 1}}`). The distinct-element count is capped
  at `MAX_LIST_LENGTH`.

### Notes

- **Element-equality reuses the W-9 comparator** (`same_element(a, b) ≡
  canonical_cmp(a, b) == Equal`): deterministic, consistent with `Sort`, and
  panic-free for `NaN` (built on `f64::total_cmp`). The type-tag tie-break keeps
  distinct numeric subtypes of equal magnitude separate, so `2` and `2.0` are
  **distinct** elements — matching Wolfram (`Union[{2, 2.}]` keeps both).
- **Two ordering families**: `Union`/`Intersection`/`Complement` sort their
  outputs; `DeleteDuplicates`/`Tally` preserve first-occurrence order.
- **DoS / cost**: outputs never exceed the sum of input lengths (already bounded by
  the W-4 input/token caps); each head re-asserts `MAX_LIST_LENGTH` defensively.
  Membership is a linear `canonical_cmp` scan (no hashing — `IRNode` carries an
  `f64` and is not value-`Hash`-keyable), so the heads are worst-case quadratic in
  the (bounded) input — a documented simplicity trade, never unbounded.
- **No grammar change**: lexer, parser, and grammar files are untouched; only the
  builtin handler table grows.

## [0.9.0] — 2026-06-19

The **W-12** deliverable (MA04 §15): Wolfram's **string builtins**, lowered onto
the *same* substrate as the rest of the lane — the string atom is already
`IRNode::Str(String)` (the W-4 lexer produces it, the printer renders it), and
`StringSplit`/`Characters` reuse the W-9 list machinery (and its
`MAX_LIST_LENGTH` cap). Like every head since W-5 these are plain `Head[args]`
applications, so there is **no grammar change**; only the `wolfram-runtime`
builtin handler table grows. The `<>` infix sugar for `StringJoin` is **deferred**
to a future grammar-change lane item.

### Added (string builtins)

- **`StringJoin[a, b, …]`** — concatenate string arguments (`StringJoin["a","b"]`
  → `"ab"`; `StringJoin[]` → `""`). DoS-capped at the new `MAX_STRING_LENGTH`
  (the running total uses `checked_add`; an over-cap join stays unevaluated
  before any allocation).
- **`StringLength[s]`** — number of **characters**, not bytes
  (`StringLength["héllo"]` → `5`).
- **`StringTake[s, n]`** — first `n` chars (`n < 0` → last `|n|`); **`StringTake[s,
  {m, n}]`** — 1-based inclusive character range. `StringTake["hello", 3]` →
  `"hel"`, `StringTake["hello", {2, 4}]` → `"ell"`, `StringTake["hello", -2]` →
  `"lo"`.
- **`StringDrop[s, n]`** — drop the first `n` chars (`n < 0` → drop the last
  `|n|`). `StringDrop["hello", 2]` → `"llo"`.
- **`StringSplit[s]`** — split on runs of whitespace; **`StringSplit[s, sep]`** —
  split on a literal string separator. Both drop empty fields and return a `List`
  of strings. `StringSplit["a b  c"]` → `{"a","b","c"}`, `StringSplit["a,b,c",
  ","]` → `{"a","b","c"}`.
- **`StringReplace[s, a -> b]`** — replace **every** non-overlapping literal
  occurrence of `a` with `b`; accepts a single rule or a `{r1, r2, …}` list of
  rules applied in sequence. `StringReplace["banana", "a"->"o"]` → `"bonono"`.
- **`ToString[expr]`** — the Wolfram surface form of `expr` via the existing
  `print_wolfram` printer; a bare top-level string renders as its **raw content**
  (no quotes), so `ToString[123]` → `"123"` and `ToString["hi"]` → `"hi"`.
- **`Characters[s]`** — list of single-character strings (`Characters["ab"]` →
  `{"a","b"}`).

### Unicode by character, never by byte

Every length, index, and slice goes through `s.chars().count()` / a
`Vec<char>` — **no byte index is ever taken** — so a multi-byte character (`é`,
an emoji) counts as exactly one position and `StringTake`/`StringDrop` can never
slice through a UTF-8 boundary (the `byte index N is not a char boundary` panic
is structurally impossible). `StringLength["héllo"]` is `5`; `StringTake["héllo",
2]` is `"hé"`.

### Safety / DoS

- New **`MAX_STRING_LENGTH`** cap (mirrors `MAX_LIST_LENGTH` = 1,000,000) bounds
  the two string-*growing* heads, `StringJoin` and `StringReplace`.
- `StringReplace` rejects an **empty pattern** (`"" -> x`, which would match at
  every position — unbounded expansion) and scans **non-overlapping
  left-to-right** (so `"a" -> "aa"` does not re-scan the inserted text; linear,
  terminating). Its output length is bounded by `MAX_STRING_LENGTH`.
- `i64::MIN` indices are handled via an `i128` magnitude (no `i64::abs`
  overflow); out-of-range / non-integer / non-string inputs leave the form
  **unevaluated** rather than panicking — the W-5/W-9 fail-soft contract.

### Tests

28 new unit tests in `builtins.rs` (each head's happy path, the Unicode cases,
the DoS caps, and the malformed-input/unevaluated paths) plus 3 end-to-end tests
in `lib.rs` (full lex→lower→eval→print, Unicode, and a malformed-input
session-survival case). `cargo clippy` clean; all `wolfram-runtime` +
`wolfram-repl` tests green.

## [0.8.0] — 2026-06-19

The **W-11** deliverable (MA04 §14): Wolfram's **pure (anonymous) functions** —
`Function[…]`, the slot forms `#`/`#n`/`##`, and the `&` postfix — the single
most-used functional idiom, so a higher-order builtin can take an inline lambda
instead of a named definition. This is the first runtime change since W-5 to
require a **grammar + lexer change** (regenerated `_grammar.rs`, mirroring W-6).

### Added (pure functions)

- **`Function[x, body]` / `Function[{x, y}, body]`** — named-parameter pure
  functions. Applying substitutes the args for the named params in the body, via
  the **same `vm.rs::substitute`** user functions, the W-7 `Table` index, and W-8
  scoping already use. `Function[x, x^2][5]` → `25`; `Function[{x,y}, x+y][3,4]`
  → `7`. A single-symbol param is normalised to a one-element list at lowering,
  so every named function is uniformly `Function[List(params…), body]`.
- **Slot forms `#`, `#1`, `#2`, …** (`#` ≡ `#1`) lowering to `Slot[n]`, and
  **`##`** (`SlotSequence`) lowering to `SlotSequence[1]`. A `##` in an argument
  position **splices** all the call's args into that argument list.
- **The `&` postfix** (`(#^2)&`, `(#1+#2)&`) turning the preceding expression
  into a slot-based `Function[body]`. `&` has a **low precedence** — looser than
  every arithmetic/comparison operator but tighter than `,` — so `#^2 &`,
  `# + 1 &`, and `Mod[#,2]==0 &` are all pure functions of the *whole* body. A
  pure function may be applied immediately (`(#^2)&[5]`), and the apply suffix
  chains (`f&[1][2]`, `f&[[i]]`).
- **`Mod[a, b]`** — a minimal integer modulo (divisor-signed remainder), the
  only new builtin W-11 needs (for the canonical `Mod[#,2]==0 &` even-predicate).

### How it composes

Application is a **rewrite rule on `Backend::rules()`**: its predicate matches a
*reducible* `Function[…][args]` (well-formed record, matching arity) and the
transform substitutes args → params/slots and returns the body for the VM to
re-evaluate. Because the rule fires inside `vm.eval`, it composes for free with
every W-5/W-9/W-10 higher-order builtin — they already re-apply `f` through
`build_canonical_application` + `vm.eval`:

- `Map[#^2 &, {1, 2, 3}]` → `{1, 4, 9}`
- `Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &]` → `{2, 4}`
- `Nest[# + 1 &, 0, 3]` → `3`

### Safety

Gating reducibility in the **predicate** (not the transform) is what prevents an
arity-mismatched / malformed `Function[…][args]` from re-matching the rule and
looping forever (a self-DoS) — a non-reducible form falls through to
`on_unknown_head` and stays unevaluated. A pure function substitutes its body
once per application (linear in the body size); self-referential recursion is
bounded by the evaluator's existing recursion handling exactly as a
self-referential `Define` is.

### Grammar (regenerated `_grammar.rs`)

New tokens `HASH` (`#`), `SLOTSEQ` (`##`, longest-match before `#`), `AMP` (`&`,
longest-match after `&&`); a `slot` atom; and a low-binding `amp` postfix level
(`amp = comparison AMP { AMP } { amp_apply } | comparison`). The `_grammar.rs`
for the lexer and parser were regenerated via the Rust grammar-tools CLI — never
hand-edited.

## [0.7.0] — 2026-06-17

The **W-10** deliverable (MA04 §13): the functional-iteration combinators — the
point-free heads every functional-programming session reaches for, lowered onto
the *same* substrate as W-5/W-9 (the `Map`/`Apply` application path
`build_canonical_application` + `vm.eval`, and the W-5 `list_elements` accessor).
All are plain `Head[args]` applications — **no grammar change** — and all are
eager (non-held), so the `WolframBackend` held set is untouched.

### Added (functional-iteration combinators)

- **`Nest[f, x, n]`** → `f` applied to `x` `n` times: `f[f[…f[x]…]]`. A symbolic
  `f` builds the literal nest (`Nest[f, x, 3]` → `f[f[f[x]]]`); a defined `f`
  reduces at each step. `Nest[f, x, 0]` is the identity (`x`).
- **`NestList[f, x, n]`** → `{x, f[x], f[f[x]], …}` — the `n + 1` intermediate
  results, including the seed.
- **`Fold[f, x0, list]`** → the left fold `f[…f[f[x0, l₁], l₂]…, lₙ]`. With
  `Plus` it totals (`Fold[Plus, 0, {1,2,3}]` → `6`); left-associative
  (`Fold[Subtract, 10, {1,2,3}]` → `4`). An empty list returns the seed.
- **`FoldList[f, x0, list]`** → `{x0, f[x0,l₁], f[f[x0,l₁],l₂], …}` — the running
  accumulations, including the seed (`FoldList[Plus, 0, {1,2,3}]` → `{0,1,3,6}`).
  An empty list returns `{x0}`.

Each combinator re-applies `f` through the **exact** `Map`/`Apply` path
(`build_canonical_application(f, args)` then `vm.eval`), so any callable resolves:
a built-in (`Plus`), a bridged head, or a user `SetDelayed` function
(`g[a_] := a + 1; NestList[g, 0, 3]` → `{0,1,2,3}`). A non-callable `f` is *not*
an error — each `f[acc]` simply stays unevaluated (`Fold[f, 0, {1,2}]` →
`f[f[0,1],2]`).

### Security / DoS

- **Iteration count `n` is capped** (`Nest`/`NestList`): `nest_count` reads `n` as
  an exact non-negative integer and refuses any `n` exceeding `MAX_LIST_LENGTH`
  (1,000,000) *before* the loop, so a tiny input like `Nest[f, x, 10^9]` cannot
  drive a billion `vm.eval` calls.
- **Result-list size is bounded**: `NestList`'s `n + 1` allocation is bounded by
  the capped `n`; `FoldList`'s `len + 1` allocation is bounded by a defensive
  `MAX_LIST_LENGTH` check on the (already source-bounded) input length. `Nest` and
  `Fold` hold only the scalar accumulator and add no result-size surface.
- Every malformed form (negative/non-integer `n`, an over-cap `n`, a non-list
  third argument to `Fold`/`FoldList`, the wrong arity) is **left unevaluated** —
  echoed back, never a panic — following the W-5 convention.

### Tests

26 new tests (14 unit in `builtins.rs`, 7 integration through the public
`eval`/`WolframSession` surface, plus edge/DoS/regression cases): the symbolic
`Nest`/`NestList` shapes, `Fold`/`FoldList` over `Plus`/`Subtract`, the degenerate
`n = 0` / empty-list cases, a user `SetDelayed` function as `f`, negative /
non-integer / over-cap `n`, non-list fold target, wrong arity, non-callable `f`,
and W-4..W-9 regression guards.

## [0.6.0] — 2026-06-17

The **W-9** deliverable (MA04 §12): list-manipulation builtins — the reordering,
concatenating, flattening, filtering, counting, and summing heads every
list-processing session reaches for. Lowered onto the *same* substrate as W-5
(the `list_elements` accessor, the `Map`/`Apply` predicate-application path, and
the canonical `Add` fold). All are plain `Head[args]` applications — **no grammar
change** — and all are eager (non-held), so the `WolframBackend` held set is
untouched.

### Added (list-manipulation heads)

- **`Sort[list]`** → ascending in the subset's documented total canonical order
  (`canonical_cmp`): numbers (by `f64` magnitude) < symbols < strings < compound
  expressions; total and stable, so it never panics and is reproducible across
  runs. Pure-numeric lists sort numerically (`Sort[{3, 1, 2}]` → `{1, 2, 3}`).
- **`Reverse[list]`** → the list reversed.
- **`Join[a, b, …]`** → two or more lists concatenated. The combined length is
  capped at `MAX_LIST_LENGTH` (checked with `checked_add` before allocating); a
  non-list argument leaves the form unevaluated.
- **`Flatten[list]`** → every nested sub-list spliced in at **all** levels;
  **`Flatten[list, n]`** → only the top `n` levels of nested sub-lists. Output
  length capped at `MAX_LIST_LENGTH`, recursion bounded by the (token-capped)
  input nesting. A negative/non-integer depth, or a non-list, stays unevaluated.
- **`Select[list, pred]`** / **`Count[list, pred]`** → keep / tally elements where
  `pred[e]` evaluates to the `True` symbol. The predicate is applied through the
  **same** path as `Map`/`Apply` (`build_canonical_application` + `vm.eval`), so a
  built-in predicate, a user `SetDelayed` function, or a bridged head all work.
  Function-predicate `Count` is the documented simplification versus full Wolfram
  pattern-matching `Count` (MA04 §12.3).
- **`Total[list]`** → the sum of the elements, folded onto the canonical `Add`
  head (consistent with W-7 `Sum` over a range); an empty list totals to `0`.

### Added (parity predicates)

- **`EvenQ[n]`** / **`OddQ[n]`** → `True`/`False` on integer parity (so
  `Select`/`Count` are testable; the W-5/W-6 surface had no predicate head).
  `rem_euclid(2)` classifies negatives correctly; a non-integer argument is
  `False` (matching Wolfram), wrong arity stays unevaluated.

### Safety / DoS (MA04 §12.4)

- `Join`/`Flatten` outputs are bounded by `MAX_LIST_LENGTH` (= `MAX_RANGE_LENGTH`,
  1,000,000), checked before allocation; `Flatten` recursion is depth-bounded.
- The size-non-increasing heads (`Sort`, `Reverse`, `Select`, `Count`, `Total`)
  add no new allocation source — their output is at most the source-bounded input.
- Every malformed form (non-list arg, non-callable predicate, bad depth, wrong
  arity) is **left unevaluated** — echoed back, never a panic — per the W-5
  convention.

### Tests

- Unit tests for each head over a real VM, plus the malformed/edge cases
  (oversize/negative depth, non-list, unbound predicate, extreme parity).
  `Select`/`Count` predicate tests run over a real `WolframBackend` so `EvenQ`
  resolves.
- End-to-end integration tests through `eval`/`WolframSession` for every
  acceptance example in the brief, a user-defined predicate, and a regression
  guard that W-4..W-8 behaviour is unchanged.

## [0.5.0] — 2026-06-17

The **W-8** deliverable (MA04 §11): local scoping — the three Wolfram heads that
bind named locals over a body. `With`, `Module`, and `Block` are lowered onto the
*same* substrate as W-7's iteration index: held heads + the `vm.rs::substitute`
primitive. No new evaluator, no opcode, no grammar change.

### Added (local-scoping heads)

- **`With[{x = e, …}, body]`** → `body` with each local bound to its **evaluated**
  RHS, substituted in and re-evaluated. Lexical and immediate, parallel binding
  (each RHS sees the surrounding scope, so a decl may reference an outer binding).
  So `With[{x = 3}, x^2]` is `9` and `With[{a = 1, b = 2}, a + b]` is `3`.
- **`Module[{x, y = e}, body]`** → lexically-scoped locals. An initialised decl
  (`y = e`) binds like `With`; an **uninitialised** decl (`x`) is α-renamed to a
  fresh gensym `x$nnn` (mirroring real Wolfram) so it stays undefined and cannot
  resolve to — or be captured by — a same-named global. `Module[{a = 1, b = 2},
  a + b]` is `3`.
- **`Block[{x = e}, body]`** → temporarily binds `x` over `body`. For the
  substitution-based subset a self-contained body is observably identical to
  `With`; `Block[{x = 5}, x + 1]` is `6`. (See §11.3 for the dynamic-scope
  simplification.)

### Binding mechanism (MA04 §11.2–§11.3)

- The three heads are **held** (added to the `WolframBackend` decorator's
  `hold_heads` set, union with the inner held set and W-7's iteration heads) so
  the declaration list and body arrive unevaluated.
- Each decl's RHS is evaluated through `vm.eval`; the collected `name → value`
  mapping is applied to a **copy** of the held body via the same `substitute`
  used for user-function parameters and the W-7 index, then the result is
  evaluated. Because the session environment is never mutated, **locals do not
  leak** (`x` is still free after `With[{x = 3}, x]`) and never clobber a global.
- Uninitialised `Module` locals are gensym-renamed (a monotonic `AtomicU64`
  counter) — the documented capture-avoidance simplification in place of full
  α-renaming of every local.

### Robustness (MA04 §11.4)

- Malformed forms are left **unevaluated**, never a panic: a non-`List` first
  argument (`With[x, body]`), a `With`/`Block` local with no value
  (`With[{x}, body]`), a non-symbol assignment target (`f[x] = 1`), or the wrong
  arity. No new allocation source — the body is substituted once per scope entry,
  bounded by the W-4 input/token caps; nested scopes recurse over strictly
  smaller bodies.

### Tests

- W-8 acceptance values; no-leak and no-clobber guards; nested scoping; a decl
  referring to an outer binding; the gensym shadow of a global by an
  uninitialised `Module` local; and the malformed-form / wrong-arity guards.

## [0.4.0] — 2026-06-17

The **W-7** deliverable (MA04 §10): iteration constructs — the first Wolfram-lane
forms that introduce a *scoped local index*. `Table`, `Do`, `Sum`, and `Product`
bind a fresh variable `i` to each value of a range and evaluate a body once per
value, lowered onto the *same* `symbolic-vm` substrate (no bespoke loop opcode,
no new evaluator).

### Added (iteration heads)

- **`Table[expr, {i, imax}]`** / **`{i, imin, imax}`** / **`{i, imin, imax, di}`**
  → the list of `expr` evaluated with `i` bound over the range. So
  `Table[i^2, {i, 3}]` is `{1, 4, 9}` and `Table[i, {i, 2, 4}]` is `{2, 3, 4}`.
- **`Do[expr, {i, n}]`** → evaluate `expr` `n` times for side effects (e.g. a
  `Set` in the body), returning `Null`.
- **`Sum[expr, {i, imin, imax}]`** → fold `+` over the range
  (`Sum[i, {i, 1, 10}]` is `55`); an empty range sums to `0`.
- **`Product[expr, {i, imin, imax}]`** → fold `×`
  (`Product[i, {i, 1, 4}]` is `24`); an empty range is `1`.

### How the index binds

- The four heads are **held** — `WolframBackend::hold_heads` now returns the
  union of the inner `SymbolicBackend` held set (`If`, `Assign`, `Define`, …) and
  `{Table, Do, Sum, Product}`, so the body and iterator spec arrive unevaluated.
- Each iteration binds `i → value` with the **same `vm.rs::substitute`** that
  binds user-function parameters, then re-evaluates the body through the VM. The
  index stays *local* (it never leaks into the session), and nested `Table`s each
  bind their own index cleanly.
- The iterator-spec *bounds* are evaluated by the handler (the head is held, so
  `{i, 1+1}` and `{i, n}`-with-`n`-bound resolve correctly), while the body
  stays held until substitution.

### DoS surface

- The per-iteration count is **capped at `MAX_RANGE_LENGTH`** (the same bound
  `Range` uses), computed in `i128` *before* any allocation or looping — an
  oversize or extreme-span iterator (e.g. `Table[0, {i, 2000000}]`) is left
  unevaluated rather than hanging or exhausting memory. `Do` is capped
  identically (the cap bounds wall-clock work, not just memory), and the cap
  composes for nested `Table`. A malformed spec (`{i}` with no bound, a zero
  step, a non-integer/non-symbol binder, or a non-list spec) stays unevaluated —
  never a panic. See MA04 §10.3.

### Notes

- No grammar/lexer change: `Table[…]`/`Do[…]`/`Sum[…]`/`Product[…]` are ordinary
  `Head[args]` applications over list-literal specs the W-1 grammar already
  parses. W-7 touches only `wolfram-runtime` (`builtins.rs` + `backend.rs`).
- `Sum`/`Product` fold onto the canonical `Add`/`Mul` IR heads, so symbolic terms
  combine through the same engine as `1 + 2` (a symbolic body like
  `Sum[x, {i, 1, 3}]` yields `x + x + x`, the engine doing no further `3x`
  normalisation — consistent with W-4 behaviour).

## [0.3.0] — 2026-06-17

The **W-6** deliverable (MA04 §9): operator sugar for the W-5 Tier-1 heads. No
new evaluation logic and no new handler — each sugar form desugars in lowering
to the exact same head the W-5 built-in table already answers, so the sugar and
its head form produce byte-identical IR.

### Added (operator sugar)

- **`f /@ x` ≡ `Map[f, x]`** — lowered by the new `lower_mapapply` over the
  parser's `mapapply` rule.
- **`f @@ x` ≡ `Apply[f, x]`** — same path; `/@` and `@@` share one
  left-associative precedence level (`g @@ f /@ x` ⇒ `Map[Apply[g, f], x]` —
  parenthesise when mixing).
- **`x[[i]]` ≡ `Part[x, i]`** — `lower_postfix` gains an `LDBRACKET` arm that
  emits `Part`; a multi-index `x[[i, j]]` folds into nested parts
  `Part[Part[x, i], j]`, and `[[ ]]` chains/interleaves with `f[…]` application
  (`x[[1]][[2]]`, `f[x][[1]]`, `Range[3][[2]]`).

So `Plus @@ {1, 2, 3}` is `6`, `f /@ {1, 2}` is `{f[1], f[2]}`,
`{a, b, c}[[2]]` is `b`, and `{{1,2},{3,4}}[[1]][[2]]` is `2`, each identical to
its long head form. Negative/out-of-range `Part` and the `Map`/`Apply`
re-evaluation behaviour carry over from W-5 unchanged.

### Notes

- `Map`/`Apply`/`Part` are **not** run through the `Plus`→`Add`-style
  `canonical_head` bridge (they are not arithmetic heads), so they reach the
  `WolframBackend` decorator handler table verbatim.
- No new DoS surface: `/@`/`@@` inherit `Map`/`Apply`'s bounds (the
  already-materialised list); `[[ ]]` only reads one element; deep `[[…]]`
  chains are parsed iteratively (bounded by the W-4 per-statement token cap), not
  by grammar recursion. See MA04 §9.4.

## [0.2.0] — 2026-06-17

The **W-5** deliverable (MA04 §8): more built-ins & evaluation, layered onto the
*same* symbolic substrate W-4 uses — no bespoke evaluator, and no edit to
`symbolic-vm`'s shared handler table.

### Added

- **`WolframBackend`** (`backend` module) — a decorator over the shared
  `SymbolicBackend`. It answers `handler_for` from a small W-5 built-in table and
  delegates everything else (`lookup`/`bind`/`on_unresolved`/`on_unknown_head`/
  `rules`/`hold_heads`, and every W-4 handler) to the inner backend. This keeps
  the new surface local to the Wolfram lane while reusing the entire evaluation
  engine, the `Plus`→`Add` bridge, user-defined functions, and `/.`.
- **List/functional/control/numeric built-ins** (`builtins` module):
  - `Length[{…}]` — element count (`0` for an atom; argument count for a non-list
    head).
  - `First` / `Last` — first/last element; **empty list left unevaluated** (no
    panic).
  - `Part[expr, i]` — **1-based** indexing; `i = 0` is the head; negative `i`
    counts from the end; out-of-range / non-integer index left unevaluated.
  - `Append[{…}, x]` — a new list with `x` appended (values are immutable).
  - `Range[n]` / `Range[a, b]` / `Range[a, b, d]` — integer ranges, **DoS-capped**
    at `MAX_RANGE_LENGTH` (1,000,000) elements *before* allocation, so a tiny
    `Range[10^9]` is left unevaluated rather than exhausting memory.
  - `Map[f, {…}]` and `Apply[f, {…}]` — re-evaluate the constructed `f[…]` through
    the VM, routing the head through the same canonical bridge as W-4 lowering
    (`build_canonical_application`), so `Apply[Plus, {1, 2, 3}]` folds to `6`.
  - `N[expr]` — coerce exact `Integer`/`Rational` to `Float`, mapping over a list
    element-wise; symbolic and already-float values pass through.
- `MAX_RANGE_LENGTH` is re-exported.
- **`If` and the comparison/logical heads** (`==`, `!=`, `<`, `>`, `<=`, `>=`,
  `&&`, `||`, `!`) already evaluated through the shared backend in W-4; W-5 pins
  them with end-to-end tests.

### Notes

- No grammar/lexer change: every W-5 head is a function-call form the existing
  `head[args]` grammar already parses. The operator *sugar* (`/@` Map, `@@` Apply,
  `[[ ]]` Part) is deferred to W-6 (MA04 §2/§4).
- All new built-ins run inside the existing W-4 worker-thread `catch_unwind`, so
  an unforeseen handler panic still becomes a clean `Err` and the session is
  rebuilt.

## [0.1.0] — 2026-06-17

Initial release — the **W-4** deliverable of the Wolfram-language lane (MA04 §7).

### Added

- `WolframSession` — a persistent, string-in / string-out runtime that lowers the
  parsed M-expression `GrammarASTNode` from `wolfram-parser` to `symbolic-ir` and
  evaluates it with `symbolic-vm`'s `SymbolicBackend`. Variable bindings (`x = 5`)
  and user-defined functions (`f[x_] := x^2`) persist across `feed` calls; the
  `Out[n]` counter persists too.
- `WolframSession::feed` (string echo) and `eval_to_outputs` (structured
  `Output`s), plus a one-shot `eval` helper.
- **Lowering** (`lower` module): the surface→IR desugaring. The head-name bridge
  maps both the infix operators and the explicit head-applications
  (`Plus`/`Times`/`Power`/`Subtract`/`Divide`/`Minus`/`Equal`/`And`/…) onto the
  canonical IR heads (`Add`/`Mul`/`Pow`/`Sub`/`Div`/`Neg`/`Equal`/`And`/…), so
  `1 + 2` and `Plus[1, 2]` evaluate identically. n-ary `Plus`/`Times` are
  left-folded into binary chains the VM folds. `Set`→`Assign`, `SetDelayed`→
  `Define` (with `x_` parameters reduced to the bound symbol for the VM's
  symbol-based parameter binding). Pattern blanks (`_`, `x_`, `_h`, `x_h`) and
  rules (`->`, `:>`) lower to the `cas-pattern-matching` `Blank`/`Pattern`/`Rule`/
  `RuleDelayed` node shapes.
- **ReplaceAll** (`/.`): a synthetic `ReplaceAll` head is intercepted before VM
  evaluation and dispatched through `cas-pattern-matching::rewrite`. A rule's RHS
  bare references to LHS-bound pattern names are rewritten into the
  `Pattern(name, Blank())` reference form the matcher's `substitute` understands.
  Supports a single rule or a `List` of rules.
- **Pretty-printing** (`printer` module): renders the evaluated IR back to Wolfram
  surface notation (infix operators, `f[…]` application, `{…}` lists), with
  precedence-aware parenthesisation so the output re-parses to the same tree.
- **Trust-boundary hardening**, mirroring `maxima-runtime`: `MAX_INPUT_LEN` (64
  KiB) input cap; `MAX_STATEMENT_TOKENS` per-statement token cap measured on the
  real `wolfram-lexer` token stream (bounding parse-tree depth so deep nesting
  cannot overflow the stack on build or drop); evaluation on a bounded
  large-stack worker thread inside `catch_unwind`, with full session rebuild after
  any caught panic. `MAX_REWRITE_ITERATIONS` bounds `/.` rewriting.

### Notes

- Scope is the W-1 grammar subset (MA04 §4): explicit `*` required (no
  juxtaposition multiplication), no `[[…]]`/`;;`/`@`/`&`/`#` etc. `Simplify`/
  `Expand` and the full `cas-*` surface are W-6.
- Built on `symbolic-ir` 0.2, `symbolic-vm` 0.20, `cas-pattern-matching` 0.1.
