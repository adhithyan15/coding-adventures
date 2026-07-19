# Changelog

## [0.1.4] - 2026-07-19

### Added

- **`tests/oracle.rs` — HML01 §7 oracle/golden testing, cross-checking
  `apl-runtime` (ground truth) against `apl_to_semantic_ir::compile_source`
  → `semantic_ir::Module` → `semantic_ir_to_javascript::compile` → a real
  `node` process.** The direct APL sibling of `matlab-to-semantic-ir`/
  `octave-to-semantic-ir`'s own `tests/oracle.rs`, completing HML01 §5's
  "APL/J's own oracle tests remain open follow-on items" note for APL (that
  spec line is updated by this PR).
  - Simpler than the MATLAB/Octave template in two ways, both verified
    empirically rather than assumed: `Case` needs no `setup`/`final_expr`
    split (APL auto-prints a bare top-level expression natively on both
    sides, unlike MATLAB's no-op `disp`), and no `normalize()` is needed at
    all (APL's display format is a literal 1:1 port between
    `apl-runtime::value::display` and `semantic-ir-to-javascript`'s
    `ArrayRt.display`, and APL comparisons never surface a JS-native
    boolean the way MATLAB's do — both confirmed by a temporary
    `ground_truth == compiled` byte-for-byte check across the whole corpus
    while drafting this file).
  - 17-case corpus: the same 9 SIR22-addendum programs `tests/e2e_node.rs`
    already proves run correctly in `node`, now ALSO cross-checked against
    `apl-runtime` for the first time (the actual point of an oracle test —
    `e2e_node.rs` only ever proved the compiled side alone doesn't crash);
    two more addendum cases (`dyadic_index_of`, `dyadic_catenate`)
    completing oracle coverage of all 9 SIR22-addendum node kinds (
    `e2e_node.rs`'s 9 tests only exercise 7 of the 9); and 6 base-cut
    cases (right-to-left/no-precedence evaluation, a true/false comparison
    pair, scalar-vector broadcast, an assignment read back by a later
    statement, and a printed 2-D matrix).
  - **Three genuine, previously-undiscovered bugs found while scoping this
    corpus, all excluded from `CORPUS` (documented in the test file's
    module doc) rather than fixed here — fixing any of them needs a change
    to `semantic-ir-to-javascript`, a separate crate, out of scope for this
    test-only PR:**
    1. Monadic `-` (negate) on a bare scalar prints the numerically correct
       value with the WRONG glyph: ASCII `-5` instead of APL's own
       high-minus `¯5`. Root cause: `runtime.rs`'s `neg` returns a plain
       native JS number for an unboxed operand, and `formatSeen`'s dispatch
       resolves `typeof v === "number"` (plain `String(v)`, ASCII) before
       it ever reaches the NDArray branch that calls `ArrayRt.display`
       (APL's own high-minus convention) — so any bare (non-NDArray) scalar
       reaching APL's auto-print path gets the wrong glyph for a negative
       value.
    2. Monadic `-` (negate) on a genuine array (rank ≥ 1, e.g. `-1 2 ¯3`)
       silently computes `NaN` instead of the correctly negated array — a
       wrong VALUE, not just a wrong glyph. Root cause: `neg`'s `numOf`
       helper only unwraps a boxed `SirFloat` or a rank-0 NDArray, never a
       genuine rank-≥1 array, so it passes an NDArray object straight
       through to native JS unary-minus, which coerces to `NaN`. Same
       failure *class* as the (already-fixed, in `matlab-to-semantic-ir`'s
       own oracle PR) while-loop/unary-minus-on-power `numOf` bug, but a
       different, still-open instance — that fix never taught `numOf`
       about a genuine multi-element array, and a proper fix for `neg`
       specifically needs real elementwise negation, not just wider
       unwrapping.
    3. Monadic `× ÷ ⌈ ⌊` (sign/reciprocal/ceiling/floor) crash with
       `TypeError: unknown builtin: <name>` for EVERY operand, scalar or
       array — confirmed live for all four. Root cause: `apl-to-semantic-
       ir`'s own README/`src/lower.rs` documents these four names as the
       intended lowering target for monadic `× ÷ ⌈ ⌊`, but
       `semantic-ir-to-javascript` never actually implements any of them —
       they are absent from both `emit.rs`'s well-known-builtin tables and
       `runtime.rs`'s `builtins` dispatch object, so the generic
       `__Sir.callBuiltin` fallback throws instead of running. Looks like
       a pure omission: these builtins were designed and documented in this
       crate's own 0.1.0 release but never given a real backend
       implementation, and no existing test anywhere exercises any of the
       four through `node`.
    - Net effect: of APL's 6 monadic-capable atoms, only `+` (a genuine
      no-op) round-trips correctly through the compiled path today; the
      other 5 (`- × ÷ ⌈ ⌊`) are all broken, in one of the three ways above.
      Reported as follow-up items (a background task was also spawned to
      track them) rather than fixed inline, per this task's explicit scope
      boundary (this PR only adds a test file to `apl-to-semantic-ir`; it
      does not touch `semantic-ir-to-javascript` or `apl-runtime`).
- Bumped to 0.1.4 (test-only addition, following this crate's own
  convention of a patch bump per dated CHANGELOG entry, established by
  0.1.2's test-only `e2e_node.rs` addition) and added a
  `coding-adventures-apl-runtime` dev-dependency (`tests/oracle.rs`'s
  ground truth) — the non-dev `[dependencies]` section deliberately still
  does NOT depend on `apl-runtime`; only this test file needs it.

## [0.1.3] - 2026-07-18

### Fixed

- **Stranded literals (`1 2 3`) now lower to a genuine rank-1 vector,
  closing the gap 0.1.2's changelog flagged and explicitly left as an
  out-of-scope follow-up** ("Discovered (not fixed here...)" below).
  Root cause was a doc-comment/implementation mismatch, not a design gap:
  `lower_term`'s multi-number ("stranding") branch built a bare
  `Expr::ArrayLit { rows: vec![row], .. }` directly, justified by a comment
  claiming "`rows.len() == 1` is precisely how a row/rank-1 vector is
  represented." That claim contradicted `semantic-ir`'s own `ArrayLit` doc
  comment (`nodes.rs`), which is explicit that a 1-row literal is a *row
  vector* — under this IR's MATLAB-derived column-major storage convention
  (`Feature::ArrayColumnMajor`), that means a genuinely rank-2 `[1, n]`
  value, not rank-1 `[n]`. This crate's own module doc comment always
  stated the correct INTENT ("`1 2 3` → one rank-1 `Expr::ArrayLit`"); the
  implementation simply picked the wrong IR node to realize it. The
  consequence: any SIR22-addendum operation correctly scoped to rank <= 1
  operands only — `outer` (`A∘.×B`) and dyadic `⍴`'s shape argument — threw
  a clean runtime `Error` in the generated JS (`outer: operands of rank > 1
  not yet supported`; `reshape: shape argument must be a scalar or vector`)
  whenever given a bare stranded literal, even though real APL accepts one
  there without hesitation.
- **The fix**: `lower_term`'s stranding branch now builds the same
  single-row `Expr::ArrayLit` as before, then wraps it in `Expr::Ravel`
  (SIR22 addendum "monadic `,A`", which already existed and already
  flattens any input rank down to a genuine rank-1 result — this crate's
  own monadic `,A` lowering already builds the identical node for the
  exact same reason). This is invisible to APL source: nothing about the
  surface syntax changes, only which IR node shape represents a stranded
  literal. No new `Feature` was needed: `Ravel` observes `MatrixOps` +
  `ArrayColumnMajor` (on top of the `NDArrays` + `ArrayColumnMajor` the
  inner `ArrayLit` already added), the same three features every other
  SIR22-addendum node in this file already observes — `lower_term` now
  adds `Feature::MatrixOps` alongside the two it already added, matching
  what `semantic-ir`'s validator independently derives when it walks the
  emitted `Expr::Ravel` node (omitting it would have made every stranded
  literal fail validation with "manifest does not declare feature
  `MatrixOps` but module uses it").
- **`tests/test_lower.rs`**: three existing tests that asserted a stranded
  literal's lowered shape directly against a bare `Expr::ArrayLit` now
  match `Expr::Ravel { target, .. }` first and drill into `**target` for
  the `ArrayLit` (or just the variant, where the row contents weren't the
  point) — `stranded_literal_is_a_single_row_array_lit` (renamed
  `stranded_literal_is_a_ravelled_single_row_array_lit_rank_1_vector` to
  reflect the new shape), `reduce_over_stranded_vector`, and
  `dyadic_rho_is_reshape_with_a_as_shape_and_b_as_target` (the `2 3` shape
  argument). Two new regression tests added:
  `outer_product_accepts_two_bare_stranded_literals_as_genuine_rank_1_operands`
  (`1 2∘.×3 4`) and
  `reshape_accepts_bare_stranded_literal_shape_and_target_as_genuine_rank_1_operands`
  (`2 3⍴1 2 3 4 5 6`), both asserting the IR shape (`Ravel`-wrapped
  operands) and clean `semantic_ir::validate`.
- **`tests/e2e_node.rs`**: two new `node`-executed tests proving the bug is
  actually fixed end to end, not just at the IR-shape level —
  `outer_product_of_two_bare_stranded_literals_runs_in_node` (`+/,1
  2∘.×3 4` → `21`) and
  `reshape_with_bare_stranded_literal_shape_and_target_runs_in_node`
  (`⍴2 3⍴1 2 3 4 5 6` → `"2 3"`), neither needing the `⍳`/`,` workaround
  the file's existing tests use. Confirmed these would have failed before
  the fix by reverting just the `lower.rs` change locally and re-running
  them: `node` threw exactly the two errors described above (`outer:
  operands of rank > 1 not yet supported (shapes [1,2], [1,2])` and
  `reshape: shape argument must be a scalar or vector (got rank 2)`). The
  file's module doc comment's "Two representational quirks" section, point
  2, is updated to record that this gap is now closed — the existing
  `⍳`/`,`-based tests are left untouched (they remain valid; they exercise
  prefix order and ravel's row-major-vs-column-major correctness, not this
  gap) since the workaround they use is harmless, just no longer
  necessary.

## [0.1.2] - 2026-07-18

### Added

- **`tests/e2e_node.rs`** — this crate's first real end-to-end,
  `node`-executed test, made possible by `semantic-ir-to-javascript`
  0.41.0 gaining real codegen for the SIR22 "APL addendum" (`Reduce`/
  `Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/
  `Ravel`/`Catenate`) — the nine node kinds essentially every non-trivial
  APL program uses. Seven tests, each compiling real APL source through
  `compile_source` → `semantic_ir::validate` → `semantic_ir_to_javascript::
  compile` → a temp `.js` file → `node`, asserting the printed stdout:
  `+/1 2 3 4` (reduce-Add) → `10`; `⌈/3 1 4 1 5` (a non-Add reduce, proving
  the op dispatch isn't hardcoded) → `5`; `-/+\1 2 3` (scan composed with a
  non-commutative reduce, which only reproduces `¯8` if the three prefix
  sums were folded in the correct left-to-right order) → `¯8`;
  `+/,(⍳2)∘.×(⍳3)` (outer product, ravelled and summed) → `18`;
  `⍴(,2 3)⍴⍳6` (shape of a reshaped matrix) → `2 3`; `⍳5` (index generator)
  → `1 2 3 4 5`; `,(,2 3)⍴⍳6` (ravel of a reshaped matrix, proving
  row-major flatten order) → `1 2 3 4 5 6`.
- Corrected this crate's own stale claim: 0.1.0's changelog entry said "No
  `tests/e2e_node.rs`: ... a real round-trip through a backend needs
  `sir-runtime-array`'s SIR22 codegen, which does not exist yet." That
  premise described the WRONG backend (`sir-runtime-array` is the
  TypeScript backend's imported npm package, separate, still not shipped)
  — the actual blocker was `semantic-ir-to-javascript`'s own INLINED
  runtime lacking codegen for the addendum, now fixed. No further action
  needed on the 0.1.0 entry itself (changelogs are a historical record,
  not edited after the fact) beyond this correction here.

### Fixed (test-only, no lowering behavior changed)

- **`tests/test_validator.rs`**: `reduce_and_outer_product_modules_
  validate_but_compile_still_rejects_them` (asserting `compile()`
  REJECTED a `Reduce`/`OuterProduct`-using module) renamed
  `reduce_and_outer_product_modules_now_compile_cleanly` and rewritten to
  assert `compile()` SUCCEEDS — the old assertion is now false, and
  leaving it in place would have made this crate's own test suite
  document a stale, no-longer-true fact about a *different* crate's
  capabilities.

### Discovered (not fixed here — out of scope, flagged separately)

- **A stranded numeric literal (`1 2 3`) lowers to a single-row
  `Expr::ArrayLit`, which this backend's (unchanged) base-cut codegen
  turns into a genuine RANK-2 `[1, n]` "row matrix" at the JS runtime-value
  level** (`__Sir.Array.fromRows([[1, 2, 3]])`), not a true rank-1 `[n]`
  vector — contrast `apl-runtime`'s OWN tree-walking evaluator, which
  builds a true `[n]` via `Array::from_vec` for the identical source (see
  `apl-runtime/src/eval.rs`). `reduce`/`scan` happen to compute identical
  numbers either way (their rank-2 branch folds/scans each row
  independently, and a lone row coincides with a rank-1 fold/scan of the
  same elements), so `+/`/`+\` on stranded literals are unaffected. `outer`
  and dyadic `⍴`'s shape ARGUMENT, however, are both scoped to rank <= 1
  operands ONLY (faithfully mirroring `array_runtime::ops::outer` /
  `apl_runtime::builtins::reshape`'s own identical restrictions) — so `1
  2∘.×3 4` and `2 3⍴⍳6` (two bare stranded literals used this way) both
  throw a clean runtime `Error` today, discovered while writing
  `tests/e2e_node.rs` above. Worked around in that test file by building
  the outer-product operands from `⍳` (which constructs a genuine rank-1
  `[n]` directly, sidestepping `ArrayLit`/`fromRows` entirely) and the
  reshape shape argument from `,2 3` (ravel of the literal, which — like
  `⍳` — always constructs a genuine rank-1 result regardless of its
  input's rank). Root cause is this crate's `ArrayLit` lowering reusing
  the MATLAB-oriented SIR22 base cut, which has no representation for "a
  true rank-1 vector distinct from a 1-row matrix" (MATLAB itself has no
  such distinction — everything is a matrix). A proper fix needs either a
  new SIR representation for a genuine rank-1 literal, or a
  targeted `apl-to-semantic-ir`-side workaround routing `OuterProduct`/
  `Reshape` operands through an implicit ravel; either is a real,
  separately-scoped follow-up, not a small patch to make inline here.

## [0.1.1] - 2026-07-16

### Changed

- **`semantic-ir-to-javascript` now accepts and correctly compiles the SIR22
  base-cut modules this frontend produces** (a bare `ElementwiseOp`, e.g.
  `3+4`) — no code change in this crate; that backend gained real codegen
  for `NDArrays`/`MatrixOps`/`ArrayColumnMajor`. `Reduce`/`Scan`/
  `OuterProduct` (this crate's own SIR22-addendum output, e.g. `+/1 2 3`,
  `1∘.×2`) remain unimplemented there and are now rejected via that
  backend's own dedicated tree-walk check rather than the plain
  feature-flag capability check (which can no longer distinguish the two,
  since they share features) — updated `tests/test_validator.rs`
  accordingly: the plain-`ElementwiseOp` test now asserts acceptance, and
  the `Reduce`/`OuterProduct` test now asserts on `compile()`'s rejection
  rather than `check_module()`'s (which no longer catches it alone).

## [0.1.0] - 2026-07-12

### Added

- Initial `apl-to-semantic-ir` frontend crate (MA-4f, per
  [HML01](../../../specs/HML01-math-to-semantic-ir.md) §2/§5) — the last
  remaining APL rollout item, and the first frontend to consume the SIR22
  addendum (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
  `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`, plus the `Max`/`Min`/`Eq`/
  `Ne`/`Lt`/`Le`/`Ge`/`Gt` `ElementwiseOpKind` variants) that `semantic-ir`
  shipped specifically for APL ahead of this crate landing.
- `compile`/`compile_source` lowering `coding-adventures-apl-parser`'s
  `GrammarASTNode` CST into a `semantic_ir::Module`.
- Supported: number literals (int/float, high-minus `¯` negative sign),
  stranded literals (`1 2 3` → a single rank-1 `ArrayLit`), variables,
  parenthesised grouping; assignment including right-associative chained
  assignment (`A←B←3`, unrolled into dependency-ordered statements); all 12
  scalar dyadic atoms (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`), unconditionally lowered to
  `ElementwiseOp` (no scalar-vs-array disambiguation needed — a genuine
  simplification over `matlab-to-semantic-ir`, since none of APL's atoms
  have a non-elementwise reading); the 6 atoms with a monadic meaning (`+ -
  × ÷ ⌈ ⌊`), mapped onto `"neg"`/`"sign"`/`"recip"`/`"ceil"`/`"floor"`
  builtins (`+` is a pass-through no-op — no complex numbers in this cut);
  `⍴`/`⍳`/`,` (shape-reshape, index-generator-index-of, ravel-catenate),
  monadic and dyadic; `/` (reduce) and `\` (scan), monadic-only; `∘.` (outer
  product), dyadic-only; auto-print of a bare top-level value expression
  onto the shared `"print"` builtin (APL's own real language semantic per
  MA05 §4, unlike MATLAB's `;`-suppression convention).
- Explicit, disclosed rejections (each a clean `AplLowerError`, never
  silently mis-lowered): the 6 comparison atoms used monadically (no
  monadic meaning in APL); a reduce/scan-decorated function used dyadically
  (both are inherently monadic); an outer-product-decorated function used
  monadically (inherently dyadic); `⍴`/`⍳`/`,` decorated with an operator
  (not scalar dyadic functions). Constructs the grammar itself cannot
  produce (boxing, the rank conjunction, user-defined functions, control
  flow) need no explicit rejection code.
- 44 tests: 40 unit tests in `tests/test_lower.rs` covering every dyadic
  atom individually, every monadic atom (valid and rejected), reduce/scan
  (valid + rejected arity), outer product (valid + rejected arity),
  `⍴`/`⍳`/`,` (monadic and dyadic, plus operator-decoration rejection),
  stranded literals, high-minus literals, chained assignment,
  first-occurrence-vs-reassignment, parenthesised grouping, undefined-
  variable rejection, parse-error propagation, and a full multi-line
  program that validates cleanly via `semantic_ir::validate`; 3 tests in
  `tests/test_validator.rs` mirroring `matlab-to-semantic-ir`'s own
  capability-rejection pattern (`semantic-ir-to-javascript` correctly
  rejects any module using SIR22/SIR22-addendum nodes, which is nearly
  every APL program).
- No `tests/e2e_node.rs`: unlike MATLAB, APL has no literal-only escape
  hatch from the array domain (every dyadic scalar op is unconditionally an
  `ElementwiseOp`), so a real round-trip through a backend needs
  `sir-runtime-array`'s SIR22 codegen, which does not exist yet (tracked
  separately, HML01 §4).
- Marks MA-4f done in `MA05-apl-language.md` §6, with a design-notes
  writeup mirroring MA-4d/MA-4e's own style.
