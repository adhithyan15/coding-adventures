# Changelog

## [0.1.2] - 2026-07-21

### Added

- **`tests/oracle.rs` — HML01 §7 oracle/golden testing, cross-checking
  `j-runtime` (ground truth) against `j_to_semantic_ir::compile_source` →
  `semantic_ir::Module` → `semantic_ir_to_javascript::compile` → a real
  `node` process.** The direct J sibling of `apl-to-semantic-ir/tests/
  oracle.rs`, completing HML01 §5's "J's own oracle tests remain an open
  follow-on item" note (that spec line is updated by this PR). 36-case
  corpus covering: right-to-left/no-precedence evaluation; a true/false
  comparison pair; a scalar-vector broadcast; assignment read back by a
  later statement; a printed 2-D matrix; reduce (`+/`)/scan (`+\`); every
  SIR22/addendum `Expr` variant this crate's own `src/lower.rs` can emit
  (`ElementwiseOp`, `ArrayLit`+`Ravel`, `Shape`, `Reshape`,
  `IndexGenerator`, `IndexOf`, `Ravel`, `Catenate`, `Reduce`, `Scan`,
  `BuiltinCall` for `neg`/`sign`/`recip`/`floor`/`ceil`/`tally`/
  `replicate`/`exp`/`print`); `@` compose; two hooks, two verb-left forks,
  and a leading-noun fork (trains — MA06 §3, J's genuinely novel feature
  with no APL precedent); and J's own leading-underscore negative-literal
  spelling (`_5`).
  - Like `apl-to-semantic-ir`'s own oracle file, needs no `setup`/
    `final_expr` split (J auto-prints a bare top-level expression natively
    on both sides, confirmed against `j-runtime`'s own module doc) and no
    `normalize()` (J's comparisons convert to plain `1.0`/`0.0` floats on
    both sides, same as APL). Unlike either MATLAB/Octave's or APL's own
    oracle file, `Case` carries one more field, `known_bug: Option<&'static
    str>`, to record entries where the compiled side is known to disagree
    with `j-runtime` due to a documented, NOT-fixed-here bug in the shared
    `semantic-ir-to-javascript` crate (see below) — `ground_truth` is still
    asserted for those entries (so a wrong corpus value is still caught),
    only the `compiled`-side assertion is skipped.

### Fixed (in this crate's own `src/lower.rs`)

Both found by this crate's own new oracle harness, and — per this repo's
established discipline for a bug genuinely local to a frontend's own
lowering — fixed directly here, not deferred:

1. **Stranded literals of 2+ numbers were never `Ravel`-wrapped**, unlike
   `apl-to-semantic-ir`'s own identical construct (that crate's 0.1.3 fix,
   which this crate shipped without). A bare `Expr::ArrayLit { rows:
   vec![row], .. }` is a genuinely rank-2 `[1, n]` value under SIR's
   column-major convention, not the rank-1 `[n]` vector a J stranded
   literal actually is — any op validating its operand is rank ≤ 1
   (dyadic `$`'s shape argument, dyadic `i.`'s haystack) rejected it
   outright. Confirmed: `2 2$1 2 3 4` (reshape whose *shape* argument is
   the stranded literal `2 2`) crashed the compiled path with `reshape:
   shape argument must be a scalar or vector (got rank 2)`, even though
   this exact program round-trips correctly through `j-runtime`. Fixed in
   `Lowerer::lower_term`, mirroring `apl-to-semantic-ir::Lowerer::
   lower_term`'s `Expr::Ravel`-wrap exactly.
2. **Monadic/dyadic `i.` silently inherited APL's 1-based `Expr::
   IndexGenerator`/`Expr::IndexOf` convention and `len + 1`-not-found
   sentinel**, genuinely wrong for J's 0-based `i.` with a plain-tally
   not-found sentinel (MA06 §1 bullet 3 — this crate's single most
   safety-critical distinction from APL). Confirmed: `i.5` compiled to
   `1 2 3 4 5` (APL's 1-based iota), not `j-runtime`'s own `0 1 2 3 4`.
   Fixed via a new `Lowerer::zero_base_index` helper that wraps both
   nodes' output in an elementwise `- 1` — an exact arithmetic identity
   for both the found and not-found cases (see that function's own doc
   comment for the proof), needing no shared-crate change at all. Updated
   `tests/test_lower.rs`'s `idot_index_generator_monadic_and_index_of_
   dyadic`, `dollar_shape_monadic_and_reshape_dyadic`, `hash_tally_
   monadic_and_replicate_dyadic`, and renamed/updated `stranded_literal_
   is_a_single_row_array_lit` → `stranded_literal_is_a_single_row_array_
   lit_wrapped_in_ravel` to match the new emitted shapes.

### Found, NOT fixed here (shared `semantic-ir-to-javascript` crate — follow-up task)

Recorded here — mirroring how `apl-to-semantic-ir`'s own oracle file
originally shipped its three bugs excluded-not-fixed before a later,
separate PR fixed them in `semantic-ir-to-javascript` 0.43.0 — rather than
patched in this PR, per this task's own scope discipline (`tests/
oracle.rs`'s module doc has the full write-up):

- **Bug A — no J-specific display convention at all.**
  `semantic-ir-to-javascript`'s `emit.rs` only ever checks
  `source_language == "apl"` to decide the negative-number/infinity glyph
  (`SIR_DISPLAY_APL_HIGH_MINUS`); there is no equivalent flag for `"j"`.
  Consequence: a bare/boxed scalar negative number or `Infinity` prints
  plain ASCII (`"-5"`, `"Infinity"`) instead of J's own leading underscore
  (`"_5"`, `"inf"`); a genuine `NDArray` (rank ≥ 1, or an already-boxed
  rank-0 value) prints APL's high-minus `¯` *unconditionally* (`ArrayRt.
  fmtNum` has no flag check at all). Confirmed against `-5`, `_5`,
  `-1 2 _3`, `-/+\1 2 3`, `(+*-)5`, and `%0`. 8 of this crate's 36 new
  oracle cases hit this and are marked `known_bug` accordingly.
- **Bug B — `tally`/`replicate`/`exp` never registered as builtins.** This
  crate's own `src/lower.rs`/README/CHANGELOG (see 0.1.0 below) document
  `#`'s monadic/dyadic forms and `^`'s monadic form as `BuiltinCall("tally"
  | "replicate" | "exp", ..)`, but `semantic-ir-to-javascript`'s builtin
  dispatch table never gained entries for any of the three — every use
  crashes with `TypeError: unknown builtin: <name>` for every operand.
  Same bug *class* as APL's own historical bug #3 (`sign`/`recip`/`ceil`/
  `floor`, fixed in `semantic-ir-to-javascript` 0.43.0), but these three
  names are new to J and were never registered at all. 3 of this crate's
  36 new oracle cases hit this. Dyadic `^` (power) is unaffected — it
  reuses the already-implemented `ElementwiseOpKind::Pow`.

## [0.1.1] - 2026-07-18

### Changed

- **`semantic-ir-to-javascript` now implements real codegen for the SIR22
  "APL addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
  `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`)** — no code change in this
  crate; `+/1 2 3` (this crate's own `Expr::Reduce` output, identical to
  `apl-to-semantic-ir`'s) used to compile down to a clean `compile()`
  rejection via that backend's now-removed dedicated tree-walk check.
  Updated `tests/test_validator.rs`'s `reduce_modules_validate_but_
  compile_still_rejects_them` (renamed `reduce_modules_now_compile_
  cleanly`) to assert `compile()` now SUCCEEDS instead of asserting the
  old rejection — mirrors the identical fix in `apl-to-semantic-ir`'s own
  `tests/test_validator.rs`. No behavioral node-execution test added
  here; `apl-to-semantic-ir`'s `tests/e2e_node.rs` is the actual
  node-executed proof for this shared codegen path (both frontends emit
  the identical `Expr::Reduce`/etc. node shapes).

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
  mechanism (or mixture) causes the depth. `lower_noun_expr` only spends
  this budget on a link whose verb is actually a duplicating `Hook`/
  verb-left `Fork` (`duplicates_monadic_operand`/`duplicates_dyadic_operands`)
  rather than unconditionally — a follow-up review caught the
  unconditional version over-counting, rejecting perfectly safe programs
  (many ordinary, non-duplicating verb applications ahead of one small
  hook) purely for chain length rather than actual duplication risk
  (regression test: `a_long_chain_of_non_duplicating_verbs_never_spends_the_combinator_budget`).
  A separate, purely defensive `MAX_TRAIN_TEETH` (64) cap bounds a single
  train's raw tooth count before any O(tooth count) collection work.
- Explicit, disclosed rejections (each a clean `JLowerError`): the 6
  comparison atoms used monadically; a reduce/scan-decorated verb used
  dyadically; `$`/`i.`/`,`/`#`/`^` decorated with an adverb (none is a
  scalar dyadic verb); a bare noun tooth anywhere except a fork's leading
  position (`j.grammar`'s own disclosed example, `(A B)`, parses
  syntactically but is semantically invalid); trains/compose nested
  deeper than the combinator-depth cap, or a single train wider than the
  tooth-count cap.
- 48 tests: 43 in `tests/test_lower.rs`, 4 in `tests/test_validator.rs`
  (mirroring `apl-to-semantic-ir`'s own capability-rejection pattern,
  extended to confirm hook/fork-using modules — ordinary nested base-cut
  applications with no new SIR node — are accepted by
  `semantic-ir-to-javascript` exactly like a plain `ElementwiseOp`
  module), 1 doctest.
