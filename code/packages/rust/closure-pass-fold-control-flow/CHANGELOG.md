# Changelog

All notable changes to the `coding-adventures-closure-pass-fold-control-flow` crate will be documented in this file.

## [0.5.0] - 2026-06-04

### Added — CLOC12.24: gap-016 `if (x) S` → `x && S` rewrite

Closes `gap-016` from the CLOC12 gap tracker. `fold_if_statement` now
has a third rewriting branch (after literal-truthy/falsy collapse and
gap-017 if-else→ternary): when the test is non-literal, the
consequent reduces to a single ExpressionStatement (directly or via
single-statement BlockStatement layers), and there is **no**
alternate, the IfStatement is rewritten to:

```
ExpressionStatement {
  LogicalExpression { left: test, op: And, right: consequent_expr }
}
```

Worked examples now folding:

```
if (x) a();              →  x && a();
if (x) { a(); }          →  x && a();   (single-expr block unwraps)
if (x) {{ a(); }}        →  x && a();   (nested single-stmt blocks)
if (x) y;                →  x && y;     (any expression statement)
```

Worked examples that stay as IfStatement (pre-conditions don't hold):

```
if (x) a(); else b();    →  x ? a() : b();   (gap-017 ternary fires
                                              first because alternate
                                              exists)
if (x) return 1;         →  unchanged (return is not an expression
                                       statement; gap-019's territory)
if (x) { a; b; }         →  unchanged (multi-stmt consequent doesn't
                                       reduce to one expression)
if (x) ;                 →  unchanged (empty consequent would require
                                       synthesising undefined; deferred)
```

### Why this is safe

`x && consequent` and `if (x) S` have observably identical evaluation
order:

* `x` is evaluated first (the short-circuit gate).
* If `x` is falsy → `&&` returns `x` without evaluating the right
  operand; `if (x) S` likewise skips `S`. Behaviour match.
* If `x` is truthy → `&&` evaluates the right operand; `if (x) S`
  likewise executes `S` for its side effects. The wrapper
  ExpressionStatement discards the result of `&&`, so the *value*
  is irrelevant — only the side effects matter. Behaviour match.

No second evaluation of `x` is introduced. `consequent`'s side
effects fire when and only when `x` is truthy.

### What changed in tests

* `tests/upstream/peephole_minimize_conditions_test.rs::test_fold_one_child_blocks_if_to_logical_and`
  — un-ignored, now exercises 4 shapes: bare `if (x) a();`,
  single-stmt block, nested blocks, and `if (x) y;`.
* `tests/upstream/peephole_minimize_conditions_test.rs::test_fold_one_child_blocks_if_else_to_ternary`
  — the trailing "testSame: no alternate" assertion was removed
  (it's now covered by the new gap-016 test) with a comment
  marking the historical pre-gap-016 behaviour.
* `src/lib.rs::if_non_literal_test_with_no_alternate_passes_through`
  — renamed to `..._with_multi_statement_consequent_passes_through`
  and the test body switched from `if (flag) x;` (now folds) to
  a 2-statement consequent block (still doesn't fold).
* `src/lib.rs::if_with_unresolved_comparison_doesnt_fold_alone`
  — renamed to `..._folds_via_gap016` and updated to assert the
  new `(1<2) && A` fold shape, while still pinning that the inner
  `1 < 2` BinaryExpression is NOT folded (fold-control-flow alone
  doesn't do binary-comparison folding; that's constant-fold).

Inline tests: 20 → 20 (two pre-existing tests renamed + updated).
Upstream tests: 10 → 11 (un-ignored gap-016 placeholder).

No public API change. No AST change. CV plumbing unchanged.

## [0.4.2] - 2026-06-01

### Changed — CLOC12.16: handle new `UndefinedLiteral` Expression variant

The fold-control-flow pass gained an `Expression::UndefinedLiteral`
arm in its expression-walk leaf list so it compiles against the new
`javascript-ast 0.6.0` AST. Behaviour: passthrough.

## [0.4.1] - 2026-06-01

### Changed — CLOC12.15 rebase: handle new `BigIntLiteral` Expression variant

The fold-control-flow pass gained an `Expression::BigIntLiteral`
arm in its expression-walk leaf list so it compiles against the
new `javascript-ast 0.5.0` AST. `literal_truthy` falls through
the wildcard (returns `None`) for bigints — we don't yet model
the `0n is falsy, anything else truthy` rule, so the if-collapse
optimisation stays conservative around bigint tests.

Bumped to 0.4.1 (rather than 0.3.3 originally planned) because this
PR was rebased on top of CLOC12.18 (0.4.0, already on main).

## [0.4.0] - 2026-06-01

### Added — CLOC12.18: if-else→ternary fold (closes gap-017)

Adds a rewrite rule in `fold_if_statement`: when the test is not a
known literal AND both branches reduce to a single ExpressionStatement
(directly or via single-statement BlockStatement layers), the
IfStatement rewrites to an ExpressionStatement wrapping a
ConditionalExpression.

Truth table:

| Input                                       | Output                  |
|---------------------------------------------|-------------------------|
| `if (x) foo(); else bar();`                 | `x ? foo() : bar();`    |
| `if (x) { foo(); } else { bar(); }`         | `x ? foo() : bar();`    |
| `if (x) foo();`                             | unchanged (no alternate)|
| `if (x) { a; b; } else c;`                  | unchanged (multi-stmt)  |
| `if (x) return 1; else return 2;`           | unchanged (return ≠ expr; tracked as gap-019) |

Side-effect safety: a ConditionalExpression evaluates `test` first
then exactly one of the two branches — identical to the if-else.

Un-ignores `test_fold_one_child_blocks_if_else_to_ternary` in the
upstream port. Updates two existing tests
(`test_if_non_literal_test_left_alone`,
`if_non_literal_test_passes_through`) to reflect the new fold.

The helper `single_expr_stmt(stmt) -> Option<Expression>` recursively
unwraps single-statement BlockStatement layers.

## [0.3.2] - 2026-06-01

### Changed — CLOC12.14: handle new `ThrowStatement` variant

The fold-control-flow pass gained a `TaggedStatement::ThrowStatement`
match arm so it compiles against the new `javascript-ast 0.4.0` AST.
Behaviour: fold the argument expression. `throw` is a definite
terminator like `return` — the dead-after-throw rule and the
`if (x) foo(); else throw e;` early-throw rewrite will land here
in follow-up gaps.

## [0.3.1] - 2026-06-01

### Changed — CLOC12.13: handle new `LabeledStatement` variant

The fold-control-flow pass gained a
`TaggedStatement::LabeledStatement` match arm so it compiles against
the new `javascript-ast 0.3.0` AST. Behaviour: recurse into the
labelled body, preserve the label verbatim. The collapse-to-empty
optimisation for `a: break a;` lives elsewhere and is tracked under
the gap-009 follow-up.

## [0.3.0] - 2026-05-31

### Added — CLOC12.05: port subset of upstream `PeepholeMinimizeConditionsTest`

Third port under the CLOC12 byte-identical contract, after
`closure-pass-constant-fold` (CLOC12.02) and `closure-pass-dce`
(CLOC12.04). Establishes the `tests/upstream/` layout for
`closure-pass-fold-control-flow`.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/peephole_minimize_conditions_test.rs` — 14 ported
  test methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.05 | **9** | **5** |

**Passing (9):** literal-test if-folds + non-literal `testSame`:

- `test_if_true_folds_to_consequent` — `if (true) x else y` → `x`.
- `test_if_false_folds_to_alternate` — `if (false) x else y` → `y`.
- `test_if_false_no_alternate_becomes_empty_statement` — `if (false) x` → `;`.
- `test_if_numeric_one_folds_to_consequent` — `if (1) x else y` → `x`.
- `test_if_numeric_zero_folds_to_alternate` — `if (0) x else y` → `y`.
- `test_if_nonempty_string_folds_to_consequent` — `if ("hi") x else y` → `x`.
- `test_if_empty_string_folds_to_alternate` — `if ("") x else y` → `y`.
- `test_if_null_folds_to_alternate` — `if (null) x else y` → `y`. (Also
  consumes the routing-gap behaviour earmarked as gap-011 in CLOC12.04
  — `if (null){x=1;}else{x=2;}` → `x=2;`.)
- `test_if_non_literal_test_left_alone` — `testSame("if (x) C else A")`.

**Ignored (5):** record upstream's broader compaction scope as new
`gap-NNN` entries:

| Test | Gap | What's needed |
|------|-----|---------------|
| `test_fold_one_child_blocks_if_to_logical_and` | gap-016 | `if (x) S` → `x && S` rewrite |
| `test_fold_one_child_blocks_if_else_to_ternary` | gap-017 | `if (x) C else A` → `x ? C : A` rewrite |
| `test_fold_conditional_de_morgan` | gap-018 | De Morgan / negation-swap rewrites |
| `test_fold_returns_into_ternary` | gap-019 | return-then-return through if-else into single ternary-return |
| `test_minimize_if_with_throw` | gap-020 | `ThrowStatement` not in Phase 1 AST |

### Cross-crate routing wins

The new `test_if_null_folds_to_alternate` passing here demonstrates
exactly what CLOC12.04's routing gaps (gap-011 / gap-012 / gap-013)
predicted: upstream's `PeepholeRemoveDeadCodeTest::testIf` line for
`null` doesn't really test DCE — it tests fold-control-flow. When
that upstream line gets re-ported into this crate (a future slice),
gap-011 can move to `RESOLVED via CLOC12.05` because the *behaviour*
is already covered here.

### Version bump

`0.1.0` → `0.3.0` (CHANGELOG already had a 0.2.0 entry from the
earlier real-body roll-out).

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body

Replaces the identity v0.1.0 body with a recursive bottom-up walker over `Program → ProgramItem → Statement → Expression`. Folds:

- **`IfStatement` with literal test** → consequent (truthy) / alternate (falsy) / `EmptyStatement` (falsy, no alternate). Truthy/falsy uses JS truthiness rules: any non-empty string / non-zero non-NaN number / true is truthy; null / 0 / "" / false is falsy.
- **`WhileStatement` with literal `false` test** → `EmptyStatement`. `while (true)` is intentionally left alone (semantics matter — infinite loops are observable).
- **Dead code after `ReturnStatement`** in `BlockStatement.body` → dropped. Recurses into nested blocks via `FunctionDeclaration.body`.
- **`ConditionalExpression` with literal test** (`true ? a : b → a`). Redundantly handled here for robustness when this pass runs solo; constant-fold also handles it.

Recurses through every Phase 1 node so deep trees are folded in one bottom-up walk.

### CV tracing — both modes work per CLOC09 amendment

- **Traced input** (`cv: Some(parent)`): the kept replacement keeps its own pre-existing `cv` (it's the same node, just promoted). A `Contribution { source: "fold-control-flow", tag: "folded-branch"|"removed-dead-code", meta: {before, after, parent_cv} }` is appended.
- **Untraced input** (`cv: None`): folds silently with no contributions. `changed: true` still set.

### Tests

19 tests (up from 8 in v0.1.0):
- pass metadata (unchanged)
- empty-program identity
- `if (true) {x} else {y} → x`
- `if (false) {x} else {y} → y`
- `if (false) {x}` no alternate → `EmptyStatement`
- truthiness across booleans, numbers, strings, null — every JS truthy/falsy case
- non-literal test (e.g. `if (flag) {…}`) passes through unchanged
- `if (1 < 2) {A}` alone does NOT fold (comparison is constant-fold's job) — documents the layering
- `while (false) {body}` → `EmptyStatement`
- `while (true)` is left alone
- dead code after `ReturnStatement` dropped (with `removed-dead-code` contribution)
- block without `return` is unchanged
- `ConditionalExpression` with truthy test folds
- **untraced mode** folds silently (no contributions)
- pipeline integration solo
- **pipeline with constant-fold registered**: `if (1 < 2) {A}` flows through both passes and ends as just `A`. Verifies the canonical CLOC06 ordering does what it's supposed to.

### Skipped (queued for v0.3.0+)
- `ThrowStatement` / labelled `BreakStatement` / `ContinueStatement` as terminators — wait for Phase 2 to add the variants.
- `while (true)` infinite-loop collapse when body is provably pure.
- `SwitchStatement` with literal discriminant — Phase 2.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — slots between `constant-fold` and `dce` in the canonical order.
- `FoldControlFlowPass` zero-sized type implementing `Pass`:
  - `name = "fold-control-flow"`
  - `depends_on = &["constant-fold"]` — folds expose statically-known conditions (`if (1+1===2)` → `if (true)`) that this pass then collapses.
  - `iteration_policy = IterationPolicy::FixedPoint` — eliminating one branch can expose another that's also statically dead.
  - `cost = 2` pass-units — matches constant-fold's weight (single tree walk with per-node local decisions).
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `FoldControlFlowPass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `IfStatement` / `WhileStatement` / `SwitchStatement` / `ConditionalExpression` nodes to fold. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 2`, `depends_on == ["constant-fold"]`, `invalidates` empty, identity run, **two-pass pipeline orders constant-fold before fold-control-flow** even when registered in reverse, **three-pass pipeline produces the canonical order** (constant-fold → fold-control-flow → dce) when all three are registered out of order, solo run with unknown deps silently dropped per the v0.1.0 scheduler, `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future side-effect attributes inform fold safety), `coding_adventures_correlation_vector` (`cv.delete()` + `"folded-branch"` `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the two-pass ordering integration test, `coding-adventures-closure-pass-dce` for the three-pass ordering integration test.
- v1 is scaffolding. The full reachability/fold logic lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put — no churn upstream.
- Followup PR: tighten `dce`'s `depends_on` from `["constant-fold"]` to `["constant-fold", "fold-control-flow"]` so the canonical order is structurally required, not incidental. Kept separate per the small-PR principle.
