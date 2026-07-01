# Changelog

All notable changes to the `coding-adventures-closure-pass-remove-unused-vars` crate will be documented in this file.

## [0.5.0] - 2026-06-30

### Added — CLOC12 upstream test port (`RemoveUnusedCodeTest`)

Ported the unused-binding cases from Google Closure Compiler's
`RemoveUnusedCodeTest.java` (the descendant of the historical
`RemoveUnusedVarsTest.java`) into `tests/upstream/remove_unused_vars_test.rs`,
following the CLOC12.01 convention (header cites the Java source; `UPSTREAM_SHA`
pins the tracked commit; `ATTRIBUTION.md` records the Apache-2.0 provenance; a
`[[test]]` entry wires the file in).

Because closurec has no public source-string → typed `Program` entry point,
each case is built directly on the typed AST with small helpers (`var_decl`,
`var_stmt`, `use_stmt`, `call`) and asserts on the surviving declarator names
after running **only** `RemoveUnusedVarsPass`.

- **11 active `#[test]`s pass**, covering the pass's whole supported surface:
  unused global `var`/`let`/`const` removal, keeping referenced bindings, the
  uninitialized (`var a;`) and pure-identifier-initializer (`var a=b;`) cases,
  multi-declarator split (`var a=1,b=2; a;` → `var a=1;`), whole-declaration
  drop when every declarator is dead, the impure-call-initializer keep, and the
  Statement-wrapped AST shape. **No defect surfaced** — the port confirms the
  pass is sound on its covered surface and adds canonical upstream coverage.
- **6 `#[ignore = "blocked on gap-NNN"]` placeholders** record upstream
  behaviors the narrow pass does not cover yet, each pinned to a new entry in
  `code/specs/CLOC12-gaps.md` (CLOC12.136): gap-121 function-local removal,
  gap-122 unused function declarations, gap-123 unused parameters, gap-124
  side-effect extraction (`var a=f();` → `f();`), gap-125 self-referential dead
  cycles, gap-126 assignment-only dead vars.

Test-only change — no production code touched. Crate version 0.4.0 → 0.5.0.

## [0.4.0] - 2026-06-16

### Fixed — the pass was a silent no-op on real programs

The CLOC13.E.1 apply step removed dead declarators only from the **bare**
`ProgramItem::Declaration(VariableDeclaration)` form. But the
`javascript-parser` bridge emits a top-level `var x = 1;` as
`ProgramItem::Statement(Statement::Declaration(VariableDeclaration))` (it routes
`variable_statement` through `Statement::Declaration`). So on every real
(bridged) program the pass matched nothing and removed nothing — and there was
no test covering actual removal to catch it (every prior test asserted
`!changed`).

This was found by wiring the pass into `closurec`'s SIMPLE pipeline and
observing `var unused = 1; used();` come out unchanged.

### Added — real removal, in both shapes, with a purity gate

- The apply step now prunes dead declarators from **both** the bare
  `ProgramItem::Declaration` form **and** the
  `ProgramItem::Statement(Statement::Declaration(...))` form, re-wrapping
  survivors in whichever shape they arrived. Shared `prune_var_decl` helper.
- **Initializer-purity gate** (`is_removable_init`): a dead declarator is
  dropped only when its initializer is side-effect-free — absent (`var x;`), a
  literal, or a bare identifier (a pure variable read). A dead binding with a
  call / member / assignment / etc. initializer is **kept**, so its side effect
  still runs. Previously the apply step deleted the whole declarator
  unconditionally, which would have silently dropped `var x = sideEffect();`.
- Removal stays restricted to `ScopeId::GLOBAL` bindings (top-level), matched by
  name against `program.body`. Function-local removal and sidecar-driven purity
  (to reach `const x = pureCall()`) remain follow-ups.

### Tests

- **Six new tests** exercising actual removal for the first time:
  `removes_unused_top_level_var_statement_form` (the bridge's shape),
  `removes_unused_top_level_var_bare_declaration_form`,
  `keeps_used_var_statement_form`, `splits_multi_declarator_dropping_only_dead`,
  `keeps_unused_var_with_impure_initializer` (purity gate), and
  `is_removable_init_classifies_purity`.

### Docs

- De-staled the module header, struct doc, `run` comments, and test-module doc,
  which all still claimed "v1 is identity" / "step 3 deferred" / "analyzer
  returns empty bindings" — none of which has been true since CLOC13.0.1.

## [0.3.0] - 2026-06-02

### Added (CLOC13.E.1 — the apply step, `changed` unpinned)

This PR is the **first apply step** lifting the `changed = false` hard-pin from any of the five CLOC13.x pass bodies. Walks `ctx.program.body` and actually removes dead bindings:

- **Build dead-name set.** Restricts `dead_bindings` (the candidate vec from step 2) to bindings with `scope == ScopeId::GLOBAL`, then collects their names into a `HashSet<String>`. Without a binding → declarator backreference (the analyzer hasn't grown one yet), we match by name + scope. Scope restriction keeps this correct under future analyzer extensions that surface nested bindings — only top-level names are ever acted on here.
- **Walk + rewrite.** Build a new `body` vec rather than `Vec::retain_mut` so the split case is straightforward:
  - `Declaration::VariableDeclaration`: partition declarators into kept vs. dropped by name.
    - All dead → drop the whole item.
    - All live → push original item verbatim.
    - Mixed → emit a new `VariableDeclaration` with surviving declarators, preserving the original's `kind` and `cv`.
  - `Declaration::FunctionDeclaration`: passthrough. `Function`-kind bindings are filtered out at step 2; function-declaration removal is the treeshake pass's job.
  - `ProgramItem::Statement`: passthrough. Statement walking lands in CLOC13.0.1 alongside references.
- **Output.** `changed = removed_count > 0`. Genuinely-mutated program is `Cow`-style returned via `ctx.program.clone()` + body replacement.

### Hard-pin lifted; safety preserved

`changed` is now derived from `removed_count`. This is safe because we genuinely mutate when we report it: zero removals → `changed = false` (identical to v0.2.0 behavior), at-least-one removal → `changed = true` (and the program is *actually* different).

**Why it stays safe under `IterationPolicy::FixedPoint`.** Each iteration reduces the binding set strictly. A removed `VariableDeclaration` produces no new bindings, so the next iteration's eligibility scan finds fewer dead entries. Fixed point reaches in at most one additional iteration after the first non-empty removal — bindings can only stop being dead by gaining a reference, which a removal never adds.

### Cross-PR interaction with CLOC13.0 (#4787)

Today (`#4787` not yet merged) the scope-analyzer still returns empty bindings. So `dead_names` is empty, the body walk runs the passthrough path on every item, `removed_count == 0`, and `changed == false`. Observable behavior is identical to v0.2.0.

Once `#4787` lands, the analyzer surfaces real top-level bindings, and any without a reference become eligible for removal. The apply step starts firing the moment the analyzer's body lands and `bindings`/`references` go non-empty. No follow-up rebase needed.

### Tests added (5 new; 15 total, was 10)

- `apply_step_passthrough_keeps_used_let_under_empty_analysis` — pins the no-op path; will fail after `#4787` lands without a referenced-`x` fixture, which is the right signal for the follow-up PR.
- `apply_step_keeps_function_declaration` — `Function`-kind bindings are never eligible (treeshake's job).
- `apply_step_passes_statements_through_untouched` — pins the deferred-Statement-walk contract.
- `apply_step_preserves_multi_declarator_when_no_dead_names` — multi-declarator passthrough.
- `apply_step_changed_is_false_when_program_unchanged` — `changed` invariant under empty-analysis.

All 10 v0.2.0 tests still pass unchanged.

### Bumped 0.2.0 → 0.3.0

API of `run` is unchanged (still `Pass::run`). Behavior under empty analysis is unchanged. The version bump signals that the pass *will* mutate the program under non-empty analysis.

## [0.2.0] - 2026-06-01

### Added — CLOC13.E: wire pass to consume `closure-scope-analyzer`

The pass `run` body now calls `scope_analyzer::analyze(program)`,
builds a per-binding use-count by scanning `analysis.references`,
and identifies bindings with use-count zero whose `kind` is `Var`,
`Let`, or `Const` (skipping `Function`/`Param`/`Class` until a
follow-up). `nodes_touched` now counts the analyzer-visited
bindings + references so the scheduler sees real cost numbers.

**Why the program is still passthrough.** The current
`scope_analyzer 0.1.0` ships an identity `analyze` (returns one
global scope with empty bindings and references — the API surface
unblocker per CLOC13). So the use-count walk finds zero dead
bindings, `removed_count` is always 0, and the program comes out
unchanged. The wiring becomes *observable* in `stats.nodes_touched`
(now counts the analyzer-visited bindings + references) and
becomes *effective* the moment the analyzer's body lands as
CLOC13.0 — no churn here.

**Step 3 (apply removal) is deferred to CLOC13.E.1.** Cleanly
dropping a binding from the AST requires a binding → declarator
backreference that the analyzer doesn't yet ship. Once it does,
the eligibility list (`dead_bindings`) feeds straight into a
walk-and-drop pass over `Program.body`.

**`changed` is hard-pinned to `false` until step 3 lands.** Under
`IterationPolicy::FixedPoint`, reporting `changed = true` while
returning an unchanged program would cause the scheduler to
re-run this pass forever (each iteration finds the same
`dead_bindings`, reports change, returns the same program, repeats).
That bug would fire the moment the analyzer's body started
populating bindings — exactly the kind of cross-PR break that's
hard to bisect. So we compute `dead_bindings` for cost-accounting
observability via `nodes_touched`, but keep `changed = false`
until CLOC13.E.1 wires actual program mutation. Security review
caught this in CLOC13.E and the fix is in this commit.

### Changed

- Cargo dependency: adds
  `coding-adventures-closure-scope-analyzer = { path = ".." }`.
- Version bumped 0.1.0 → 0.2.0 (additive runtime behavior change;
  no API surface change on `RemoveUnusedVarsPass`).

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — the final cleanup pass. Deletes variable bindings whose initializer is pure and whose reference-count after DCE + inline have run is zero. Closure Compiler ships an analogous `removeUnusedVars` pass for the same reason.
- `RemoveUnusedVarsPass` zero-sized type implementing `Pass`:
  - `name = "remove-unused-vars"`
  - `depends_on = &["dce", "inline"]` — DCE prunes unreachable code that may have referenced bindings; inline replaces call sites and can leave function declarations unreferenced. Both must run first to catch the maximum orphan set.
  - `iteration_policy = IterationPolicy::FixedPoint` — removing one binding can unreference another (chain of pure initializers). Bounded in practice by chain length.
  - `cost = 3` pass-units — per-scope binding-table build + delete. Same shape as DCE.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `RemoveUnusedVarsPass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `VariableDeclaration` / `Identifier` nodes to remove. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 10 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 3`, `depends_on == ["dce", "inline"]`, `invalidates` empty, identity run, **two-pass pipeline orders dce before remove-unused-vars** even when registered in reverse, **three-pass pipeline canonical-orders dce + inline before remove-unused-vars** when registered out of order, solo pipeline run (with the v0.1.0 `pipeline.fixed-point-not-yet-iterated` diagnostic asserted), `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (`pure` / `no_side_effects` attributes inform delete safety), `coding_adventures_correlation_vector` (per-deletion `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-dce` for the two-pass ordering integration test, `coding-adventures-closure-pass-inline` for the three-pass ordering integration test.
- v1 is scaffolding. The full per-scope walk + deletion lands once `javascript-ast` grows the needed variants. The public surface stays put — no churn upstream.
