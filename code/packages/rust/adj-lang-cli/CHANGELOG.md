# Changelog

## [0.6.0] - 2026-06-14 — relational recall: binding-query `"recall"` output (MYCIN-2026 REL-3)

### Added

- **`"recall"` JSON section** for relational recall binding queries. A query goal
  containing a `$variable` (`? deficient_in(tay_sachs, $Enzyme)`) is routed out of
  the differential and resolved by SLD enumeration over the grounded knowledge
  graph; each answer reports its variable **bindings** and the **citations** of
  the edge(s) that prove it (source/locator/trust). An empty answer set is
  explicit **abstention** (`"abstained": true`) — no grounded edge supports an
  answer, so none is fabricated. 0 answer-time model calls.
- Ground hypothesis queries are unaffected (still flow to the differential
  `ranked`/`decision`); the `"queries"` echo lists every query (ground + binding).

## [0.5.0] - 2026-06-12 — `import`-aware compile + the filesystem trust boundary (MYCIN-2026 M3)

### Added

- **The CLI now resolves `import`s before compiling.** A program may `import`
  sibling `.adj` files (dictionary ← rulebook ← case); the CLI walks the graph
  via `adj_lang::compile_with_imports` and emits the decision over the composed
  program. Import errors (cycle, bound, missing/unparseable/escaping file)
  surface as a single `{"error": …}` line.
- **`FsProvider` — the import trust boundary.** The `adj-lang` library does no
  I/O; this filesystem-backed `ImportProvider` is the only thing that reads
  disk, so all path safety lives here: canonical ids are absolute,
  symlink-resolved real paths (so spellings dedupe and a symlink can't forge a
  second identity); import literals must be **relative** (absolute refused); the
  resolved real path must stay within the **sandbox root** (the program file's
  directory) — `../…` escapes and symlinks pointing outside the root are refused,
  so `import` cannot read arbitrary host files.
- 5 e2e tests (3-file decide; diamond no-duplicate; traversal / absolute / cycle
  all refused without hang).

## [0.4.1] - 2026-06-12 — `FromSolve` proof: the solver certificate renders under the verdict step (ADJ constraints E3)

### Added

- **The verdict's proof now descends into the solver certificate.** When a
  contribution fired from a constraint STATUS atom (E2 — `feasible` /
  `infeasible` / `solved` / `optimal` / `unbounded`), its proof step now carries
  a `"solver": …` field with that constraint's full result — the **IIS `core`**
  for `infeasible`, the assignment for `solved`, the value + binding constraints
  for `optimal`, etc. So `schedule_broken`'s proof step reads
  `…,"solver":{"outcome":"unsat","core":[0,1,2]}` — the verdict, *and* the exact
  conflicting constraints that forced it, in one auditable tree.
- Implemented entirely in the CLI renderer (no `logic-engine` change): a single
  `status_certificates` helper produces the `(status atom, certificate JSON)`
  pairs that both feed the differential (E2) and annotate the proof (E3); the
  certificate JSON reuses the existing `check_json`/`solve_json`/`optimize_json`
  renderers. 4 golden tests (IIS core / solved assignment / optimum under the
  step; no `solver` field on an ordinary contribution).

## [0.4.0] - 2026-06-12 — feed-a-verdict: constraint outcome drives the differential (ADJ constraints E2)

### Added

- **The constraint engine now feeds the differential — one engine, not two.**
  The CLI runs `solve`/`check`/`optimize` *first*, maps each outcome to a STATUS
  atom, and injects it as an observed fact into the KB *before* `decide` runs:
  - `check` Sat/SatReal → `feasible`; Unsat → `infeasible`
  - `solve` Solved/SolvedRoots → `solved`
  - `optimize` Optimal → `optimal`; Infeasible → `infeasible`; Unbounded → `unbounded`
  - Unknown / Unsupported / NoUniqueSolution → **nothing** (the engine never
    launders an undecided constraint into a verdict).
- An existing `contributes <lr> from <status> to <verdict>` clause then fires in
  the differential — composing solver result → verdict through the ordinary
  contribution + proof machinery, with **no new engine logic and no grammar
  change**. E.g. an infeasible schedule drives `schedule_broken`; loosening one
  deadline makes it feasible and the same program drives `schedule_ok` instead.
- 4 golden tests (infeasible `check` → verdict, feasible → the other verdict,
  infeasible LP → verdict, inert when no status clause references it).

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
