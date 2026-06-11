# Changelog

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
