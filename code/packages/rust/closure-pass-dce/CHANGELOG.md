# Changelog

All notable changes to the `coding-adventures-closure-pass-dce` crate will be documented in this file.

## [0.4.2] - 2026-06-01

### Changed — CLOC12.14: handle new `ThrowStatement` variant

The DCE pass gained a `TaggedStatement::ThrowStatement` match arm
so it compiles against the new `javascript-ast 0.4.0` AST.
Behaviour: recurse into the argument expression. `throw` is a
definite terminator — the dead-after-throw collapse inside
`BlockStatement` (analogous to dead-after-return) is a follow-up
gap; this PR only adds the structural walk.

## [0.4.1] - 2026-06-01

### Changed — CLOC12.13: handle new `LabeledStatement` variant + un-ignore the upstream test

The DCE pass gained a `TaggedStatement::LabeledStatement` match arm
so it compiles against the new `javascript-ast 0.3.0` AST.
Behaviour: recurse into the labelled body (so dead-after-return
inside `a: { ...return... ...dead... }` still gets stripped),
preserve the label verbatim. The collapse-to-empty optimisation
for `a: break a;` is a separate gap.

The previously-ignored upstream test
`test_remove_no_op_labelled_statement` is now un-ignored — it
builds the `a: break a;` AST by hand and asserts DCE leaves it
alone (the *current* behaviour). When the collapse optimisation
lands, the assertion flips from `assert_dce_same` →
`assert_dce_yields(..., vec![])`. The upstream-test passthrough
table in this CHANGELOG that previously listed
`test_remove_no_op_labelled_statement | gap-009` should now read
`gap-009 (AST modelled; collapse follow-up)` — see
`code/specs/CLOC12-gaps.md`.

## [0.4.0] - 2026-05-31

### Changed — CLOC12.06: gap-011 marked RESOLVED via cross-crate routing

Bookkeeping PR. The `test_if_with_constant_test_collapse` stub in
`tests/upstream/peephole_remove_dead_code_test.rs` was previously
`#[ignore]`-ed with `gap-011`, on the rationale that upstream
`PeepholeRemoveDeadCodeTest::testIf` lines like `if (1){…}` →
consequent really belong in `closure-pass-fold-control-flow`'s
territory in our setup.

CLOC12.05 (PR #4672) ported the matching upstream behaviour into
`closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
as `test_if_true_folds_to_consequent`, `test_if_false_folds_to_alternate`,
`test_if_null_folds_to_alternate`, etc. — all passing. The
behaviour is covered in the right crate.

This PR closes the loop:

- The DCE-side stub no longer carries `#[ignore]`. It now passes as
  a marker test whose body documents where the actual behavioural
  coverage lives. This keeps the cross-crate audit trail explicit
  for future readers searching upstream's `testIf` method name.
- `code/specs/CLOC12-gaps.md` marks gap-011 `RESOLVED in CLOC12.06`
  with the full list of fold-control-flow ports that cover the
  behaviour.

### Port score (this crate)

|             | passing | ignored |
|-------------|---------|---------|
| CLOC12.04   | 5       | 7       |
| **CLOC12.06** | **6** | **6**   |

### Version

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-05-31

### Added — CLOC12.04: port subset of upstream `PeepholeRemoveDeadCodeTest`

Second port under the CLOC12 byte-identical contract. Establishes the
`tests/upstream/` layout for `closure-pass-dce`, mirroring the
`closure-pass-constant-fold` layout from CLOC12.02.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/peephole_remove_dead_code_test.rs` — 12 ported test
  methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.04 | **5** | **7** |

**Passing (5):** test the two narrow categories DCE actually
implements today:

- `test_function_with_bare_return_is_unchanged` — `foldSame(\"function f(){return;}\")`.
- `test_function_with_assignment_then_return_is_unchanged` — equivalent of
  `foldSame(\"function f(){x=3;return;}\")`, used to verify that
  not-unconditionally-terminating sequences stay put.
- `test_dead_statement_after_return_with_argument_is_dropped` —
  `function f(){return 3;foo();}` collapses to `function f(){return 3;}`.
- `test_multiple_dead_statements_after_return_are_dropped` —
  multi-statement tail after a bare return all get dropped.
- `test_empty_statements_dropped_from_function_body` — equivalent of
  `fold(\"{x=3;;;y=2;;;}\", \"x=3;y=2\")` applied at the function-body block.

**Ignored (7):** record upstream's broader scope as `gap-NNN`
entries that we expect to address in other passes or future AST work:

| Test | Gap | What's needed |
|------|-----|---------------|
| `test_remove_no_op_labelled_statement` | gap-009 | `LabeledStatement` / `BreakStatement` AST nodes |
| `test_fold_block_flattening` | gap-010 | nested-block flattening (single-child collapse) |
| `test_if_with_constant_test_collapse` | gap-011 | belongs in `fold-control-flow`, ported there |
| `test_hook_cleanup` | gap-012 | belongs in `constant-fold`, ported there |
| `test_fold_useless_loop_body` | gap-013 | belongs in `fold-control-flow` |
| `test_optimize_switch` | gap-014 | `SwitchStatement` AST not in Phase 1 |
| `test_var_lifting` | gap-015 | belongs in `remove-unused-vars` / hoisting pass |

### Why most tests are ignored

Upstream `PeepholeRemoveDeadCode` is much broader than our DCE pass.
It collapses dead `if` branches, simplifies useless loops, optimizes
switches, removes useless labelled statements, normalises `let`/
`const`/`var` lifting, etc. In our setup those responsibilities
mostly live in other pass crates:

- Dead-branch / loop-body simplification ⇒ `closure-pass-fold-control-flow`
- ConditionalExpression cleanup ⇒ `closure-pass-constant-fold`
- Unused variable / scope-based pruning ⇒ `closure-pass-remove-unused-vars`

So these ports stay marked `#[ignore]` in *this* file and will be
re-ported into the matching pass crates' `tests/upstream/` directories
in future CLOC12 slices. The gaps make the cross-crate routing
explicit.

### Version bump

`0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body (final step of the autonomous chain)

Replaces v0.1.0's identity body with a recursive walker over `Program → ProgramItem → Statement → Expression`. Final step in the autonomous-chain real-body rollout (after constant-fold, fold-control-flow, and the closure-emitter).

Two cleanup categories per `BlockStatement.body`:

- **Dead-after-terminator**: drop everything after a `ReturnStatement`. Phase 1 doesn't have `ThrowStatement` yet; `BreakStatement` / `ContinueStatement` only qualify in their enclosing loop scope (Phase 2 work).
- **Empty-statement removal**: drop `EmptyStatement` nodes entirely. They're semantically a no-op (`;`) and clutter output.

Recurses through every Phase 1 node so nested blocks (function bodies, if-bodies, while-bodies, for-bodies) get cleaned too. Records one `Contribution` per drop *category* per block (not per-statement — that'd be too noisy).

### Why this overlaps with fold-control-flow's dead-after-return

Intentional overlap:

- `fold-control-flow` does the cleanup as part of its block rewrite when it observes the terminator while folding.
- DCE runs **after** fold-control-flow per CLOC06 canonical order, and catches:
  - Cases where fold-control-flow didn't enter the block (e.g. it was busy folding the surrounding `if`'s test);
  - `EmptyStatement` nodes that fold-control-flow *produced* when it collapsed `if (false) { … }` with no alternate;
  - Future cases where a Phase 2+ pass leaves dead code behind.

### CV tracing — both modes per CLOC09 amendment

- **Traced** (`cv: Some` on the block): `Contribution { source: "dce", tag: "removed-dead-code" | "removed-empty-statement", meta: {before, after, parent_cv} }` appended per category that triggered.
- **Untraced** (`cv: None`): drops silently, no contributions emitted. `changed: true` still set.

### Tests

14 tests (up from 8 in v0.1.0):
- pass metadata unchanged
- empty program identity
- `{x; return; y; z;}` → `{x; return;}` (drop y and z)
- `{x; y;}` (no return) unchanged
- `{x; ; y; ; ;}` → `{x; y;}` (drop empties)
- Both categories in one block: `{x; ; return; y; ;}` → `{x; return;}` with two contributions
- Nested blocks: outer kept, inner's dead-after-return cleaned
- Untraced mode drops silently
- Pipeline solo
- **Full canonical pipeline**: `constant-fold + fold-control-flow + dce` on `if (1 < 2) {z;}` → `z;` (the whole chain cooperates correctly)
- **End-to-end through the chain**: `function f() { if (false) {x;} return; y; }` → `function f() { return; }` after fold + fold-control-flow (drops if-false-branch and creates EmptyStatement, drops y after return) + dce (removes the leftover EmptyStatement)

### Dependencies
- Added `coding-adventures-closure-pass-fold-control-flow` as a dev-dep for the full-canonical-pipeline test.

### Skipped (Phase 1.x / 2+)
- Unreferenced `VariableDeclaration` removal — `closure-pass-remove-unused-vars`'s job.
- Empty `BlockStatement` collapse to `EmptyStatement` — preserves debugging-step shape for now.
- Phase 2: `ThrowStatement` as terminator, `BreakStatement` / `ContinueStatement` qualifying in their loop scope.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — second concrete optimization pass after constant-fold.
- `DcePass` zero-sized type implementing `Pass`:
  - `name = "dce"`
  - `depends_on = &["constant-fold"]` — folds expose dead arms so they run first per CLOC06 canonical order. `fold-control-flow` will join this list once it exists.
  - `iteration_policy = IterationPolicy::FixedPoint` — deletion can free further nodes.
  - `cost = 3` pass-units (tree walk + reachability marking + post-walk deletion).
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `DcePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to delete. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy` is FixedPoint, `cost` is 3, `depends_on == ["constant-fold"]`, `invalidates` empty, run on empty Program is identity, **pipeline correctly orders constant-fold before dce** even when DCE is registered first (this is the key value-add of the depends_on edge), DCE runs as a solo pass with unknown deps silently dropped per v0.1.0 scheduler, `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future `pure` / `no_side_effects` attributes inform deletion safety), `coding_adventures_correlation_vector` (`cv.delete()` + `"deleted"` `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the ordering integration test.
- v1 is scaffolding. The full reachability walk + deletion lands once `javascript-ast` grows the needed variants. When that happens, the `Pass::run` body changes but the public surface stays put — no churn upstream.
