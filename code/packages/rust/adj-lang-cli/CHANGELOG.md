# Changelog

## [0.3.7] - 2026-06-11 — decompose→solve golden tests (ADJ constraints track D2)

### Added

- `tests/decompose_run.rs` — deterministic golden tests over the **committed**
  decompositions from the live decompose→solve demonstration
  (`code/specs/data/adj-constraints-decompose-run/`). A local model
  (`llama3.1:8b`) turned 4 messy-prose word problems into adj-lang; the engine
  solved 3/4 to gold (44, 980, unsat) at **0 answer-time model calls**, and
  reported the 4th (a model mis-transcription) as `unsupported` rather than
  fabricating a value. The tests re-solve every committed `.adj` with no model in
  the loop — proving the engine is a pure function of the decomposition. Test +
  spec-data only; no CLI behavior change.

## [0.3.6] - 2026-06-11 — grant-allocation LP worked example (ADJ constraints track D)

### Added

- `grant_allocation.adj` worked example + golden test: an allocation LP
  (`maximize 3·outreach + 2·training` under a budget + capacity) solved
  end-to-end through the CLI at **0 model calls** → optimal 26 at (6, 4),
  binding constraints [0, 1]. Demonstrates the C2 `optimize` path in the
  worked-examples suite (now 5 examples). Test-only; no CLI behavior change.

## [0.3.5] - 2026-06-11 — emit LP optima (ADJ constraints track C2)

### Added

- When a `.adj` program declares a `minimize`/`maximize` objective, the CLI calls
  `adj_constraint_solver::optimize` and emits an `optimize` section:
  `{"outcome":"optimal","value":…,"assignments":[…],"binding":[…]}`, or
  `unbounded` / `infeasible` / `unknown`. A program with no objective emits no
  `optimize` key. 4 new golden tests. Tracks `adj-constraint-solver` 0.6.0.

## [0.3.4] - 2026-06-11 — emit real-feasibility verdicts (ADJ constraints track C1)

### Added

- Render the new `FeasibilityOutcome::SatReal` as
  `{"outcome":"sat_real","assignments":[{"name","value"}]}`, where `value` is a
  rational witness rendered as a JSON number. A `check` over a fractional or
  integer-infeasible-but-real system (`2 * x = 1` → `x = 0.5`) now emits
  `sat_real` instead of `unknown`. 2 new golden tests. Tracks
  `adj-constraint-solver` 0.5.0.

## [0.3.3] - 2026-06-11 — emit feasibility verdicts (ADJ constraints track B2c)

### Added

- When a `.adj` program ends with `check`, the CLI calls
  `adj_constraint_solver::check` and emits a `check` section:
  `{"outcome":"sat","assignments":[…]}` (with a witness integer per symbol),
  `{"outcome":"unsat","core":[…]}` (the conflicting constraint indices), or
  `{"outcome":"unknown","reason":…}`. A solve-only program emits no `check`
  key. 3 new golden tests (sat witness, unsat conflict, no-check). Tracks
  `adj-constraint-solver` 0.4.0.

## [0.3.2] - 2026-06-11 — emit nonlinear roots (ADJ constraints track C3)

### Added

- Render the new `SolveOutcome::SolvedRoots` as
  `{"outcome":"solved_roots","var":…,"roots":[…],"from_constraints":[…]}`, so a
  nonlinear single-unknown equation (`constrain x * x = 4`) emits its real roots
  (`[-2, 2]`). 1 new golden test.

## [0.3.1] - 2026-06-11 — solver substitutes observed facts (ADJ constraints track B3)

### Changed

- The `solve` call now passes the KB (`solve(&lowered.constraints, &lowered.kb)`),
  so a constraint that references an observed fact is solved with that fact's
  value substituted (`adj-constraint-solver` 0.2.0). 1 new golden test.

## [0.3.0] - 2026-06-11 — constraint solving in the CLI (ADJ constraints track B2b)

### Added

- When a `.adj` program declares a constraint system (`symbol` / `constrain` /
  `solve for`), the CLI now calls `adj_constraint_solver::solve` and emits a
  **`solve`** section in the JSON output:
  - `{"outcome":"solved","assignments":[{"name","value"}],"from_constraints":[…]}`
    — solved values, each cited to the constraints that determined them.
  - `{"outcome":"no_unique_solution"}` (singular / non-square), or
    `{"outcome":"unsupported","reason":…}` (inequality, non-linear term,
    aggregation) — **never a fabricated answer**.
  - The `solve` key is omitted entirely for a pure prior/contributes rulebook.
- New dependency on `adj-constraint-solver`. Linear-equality systems only this
  slice; feasibility (`check` → SAT/UNSAT via `constraint-engine`) and
  optimization follow.

## [0.2.0] - 2026-06-10 — predicate proof steps

### Added

- Render the new `predicate` proof-step kind in the JSON proof DAG:
  `{"kind":"predicate","slot","op","threshold","observed","logit",
  "source","locator","trust"}`. A predicate-gated contribution that fires
  shows the **literal comparison the engine evaluated on the CPU**
  (`gross_income >= 14600`, observed `18000`) alongside its citation — the
  byte-cited audit trail for a deterministic rule, reconstructable without
  the model.

## [0.1.0] - 2026-06-10 — initial CLI driver

### Added

- `adj-lang-cli PROGRAM.adj` — the CPU-bound reasoner entry point for the adj-lang
  DSL. Reads a `.adj` program (rulebook clauses + `observe`/`?` lines), compiles it
  via `adj_lang::compile`, runs `adj_lang::decide` (the `logic_engine` differential),
  and emits JSON: the ranked hypotheses with per-step proof DAGs, and the decision
  (`determinate` / `kickback` / `empty`). **Zero model calls.**
- Each proof step is joined back to its firing clause and emits the cited
  `source` / `locator` / `trust` tier — the byte-cited audit trail, reconstructable
  without the model. Non-finite numbers serialize as JSON `null`.
- Declarative argument parsing via `cli-builder` (embedded JSON spec). Exit codes:
  0 ok, 1 compile error (`{"error": ...}`), 2 bad args / unreadable file.
- 3 golden tests (`tests/cli_golden.rs`): single-hypothesis cited proof DAG,
  two-hypothesis differential ranking + decision, compile-error-as-JSON.
