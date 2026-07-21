# Changelog

## [0.15.0] — 2026-07-20 — the unified reasoning trace (RS-4 PR-B)

Implements the ordered/addressed/self-contained step contract of
`ADJ-REASON-MATH.md` §E.1–E.2. You could already ask ADJ *what* it concluded and
*what it cited*; now you can ask **how it got there**.

### Added

- **`trace_steps_json` — a TOTAL walker over `DerivationOrigin`.** Renders a
  proof as an ordered list of steps, each carrying `step` (index), `depth`
  (nesting), the goal, and its **inline resolved provenance** — not a `FactId`
  pointer. The trace is therefore self-contained: it can travel to a reviewer or
  another machine and still be readable without the KB that produced it.
  The match has **no wildcard arm**. The previous renderer ended in `_ => {}`,
  which would silently discard four of six step kinds. That arm was latent in
  shipped paths (the likelihood-ratio kinds are rendered by `proof_json`), but a
  wildcard that drops reasoning is a trap for whoever adds the next step kind —
  as `FromNegation` immediately proved. Adding a variant now breaks the build,
  which is the point.
- **`steps` on `recall` and `lookups` answers.** Both previously rendered only
  from `via_facts`, which is `sort()`ed and deduplicated — a citation SET that
  says which sources were involved but never in what order or through which
  rules. `citations` is kept unchanged for existing consumers; `steps` is the
  derivation beside it.
- **`derivation` on every `derived` value.** The engine builds a
  `DerivationNode` tree for every `let` and every formula application, and the
  CLI **dropped it at the JSON boundary** — `derived_json` never read `.tree`.
  It is now emitted: `op` nodes show the arithmetic, and `leaf` nodes resolve
  through their real `FactId` to the source fact's provenance. That
  compute-to-bytes bridge already existed inside the engine; this is what makes
  it reachable from outside the process.

### Notes

- Additive only: no existing field changed or was removed, and a program with no
  `let`, recall, or lookup produces byte-identical output.

## [0.14.0] — 2026-07-18 — table answers now cite the row that produced them (ADJ-TABLES RS-5e)

### Changed

- **A table-backed answer's `citations` now quote the span defending *that row*** — not the table's
  single envelope. Ask the shipped `environment/air-quality-index` table for AQI 120 and the answer
  cites *"Orange Unhealthy for Sensitive Groups 101 to 150 …"*; previously it cited the `0 → good`
  sentence regardless of which band was selected. This holds for **exact** recall and **range**
  lookup alike, and flows through the proof DAG's `via_facts` the same way.
- **No CLI source changed.** Each row already lowered to its own `Fact`, and the citation path
  already cited *the fact that produced the answer*; adj-lang 0.55.0 simply gives that fact the
  row's provenance. The version bump records the observable change in emitted citations.
- Tables whose rows carry no provenance block are byte-for-byte unchanged.

## [0.13.0] — 2026-07-18 — RS-5c: range / bracket lookup answers (ADJ-TABLES)

### Added

- **`"lookups"` output section** for range/bracket lookups. Each `? lookup <table> <key_col> = <n>
  mode range give <value_col>` resolves the table (its rows are its facts) by enumerating the
  relation, selects the breakpoint row whose key is the greatest key `<= n`, and emits
  `{query, mode, answers:[{bindings, citations}], abstained}` — the value column bound, the
  **matched breakpoint key** named in the audit, and the selected row's citation carried through
  (the same `via_facts → provenance` flow as exact recall). A query **below the smallest key**
  honestly `"abstained": true` — "below the table's domain", never a fabricated classification.
- The comparison rides the exact `BigRational` order (`ExactRational::as_ratio()` — the identical
  total order the engine's `CmpOp` exact path uses), so there is **no `f64` hop** in the decision.
  0 answer-time model calls. The section is omitted entirely when the program declares no
  `? lookup … mode range …`, so existing output is byte-for-byte unchanged.

## [0.12.0] — 2026-07-14 — exact-number arc COMPLETE: computed results render every digit (ADJ-EXACT-NUMBERS NX-4)

### Changed

- **`derived.value` now renders exact-first.** When a `let`/`formula` computation stayed inside
  exact rational arithmetic (NX-3) and its result has a finite base-10 expansion, the CLI prints
  **all** of its digits (via the new `ExactRational::to_exact_decimal_string`), mirroring NX-2's
  exact recall-binding rendering. So a stored 39-digit π fed through the shipped `product` formula
  and doubled now renders `6.283185307179586476925286766559005768394`, not the f64-truncated
  `6.283185307179586`. The `f64` (`jnum`) remains the labeled-lossy fallback, used only when there
  is no exact sidecar or when the value **repeats** (e.g. `1/3`), which no finite decimal can hold.
  The field stays a JSON number literal — its type is unchanged; only its precision grows for values
  that were previously truncated. Non-terminating results (e.g. the ideal-gas pressure, a division
  whose denominator carries a prime other than 2/5) are byte-for-byte unchanged.

### Added

- E2E proofs closing the arc: the shipped `mathematics/constants.adj` binds π, e, and
  golden_ratio to their **full** published digit strings through the CLI (updated from the previous
  ~16-digit leading-substring assertion, which hedged that "the runtime binds a double-precision
  float" — no longer true); and a computed-result test doubles a stored 39-digit π through the
  shipped `product` formula and asserts the rendered `value` shows all 39 exact digits.

## [0.11.1] — 2026-07-14 — exact-number bindings render every digit (ADJ-EXACT-NUMBERS NX-2)

### Added

- E2E test: a native `table` whose cell is π to 39 decimal places binds through `? t(key, $V)`
  and renders **all 39 digits**, proving the exact-numbers path survives parse → store → query →
  render end-to-end (previously it came back as the f64-truncated `3.141592653589793`).

### Changed

- Behavior: a numeric literal whose magnitude overflows `f64` (`1e400`) now compiles and is stored
  exactly, instead of being reported as a malformed-literal error. The `malformed_numeric_clauses`
  golden was updated to probe a scale-amplification payload (`1e-2000000000`), which `BigDecimal`'s
  `MAX_SCALE` budget still rejects cleanly — so the "un-representable literal → `{"error":…}`, never
  a panic" invariant is unchanged.

## [Unreleased] — render applied-formula provenance in the `derived` section

### Added

- A `derived` value produced by APPLYING a provenanced `formula`
  (ADJ-FORMULA-LIBRARIES rung-0) now renders the formula's cited
  `source`/`locator`/`trust`/`corroborations` alongside its `name`/`value`/`dim` —
  the audit channel proving WHY the formula is trusted, beside the derivation tree
  proving HOW the number was computed. A plain `let` (no library claim) omits the
  field, so existing output is byte-for-byte unchanged.

## [0.11.0] - 2026-06-29 — surface `let`-derived dimensioned values

### Added

- New optional `"derived"` JSON section: one object per `let`-bound value with
  its `name`, computed `value`, exact rational (when integer/rational), and the
  `dim` tag the engine **inferred** (`"km/h"`, `"mol/l"`, `"scalar"`, …). The
  tag is formed by the engine's `Dimension::combine` at each operation, never
  written by the model — so a grader can reject a numerically-right-but-unit-wrong
  answer. The section is **omitted entirely** when a program binds no `let`, so
  existing rulebook/recall output is byte-for-byte unchanged. Backs the
  ADJ-LADDER `rung4_dimensional` rung's `compute_dimensioned` extractor.

## [0.10.0] - 2026-06-28 — render derived evidence proofs

### Added

- LR contribution proof JSON now includes an `"evidence_proof"` array when the
  contribution fired from a rule-derived evidence atom. The nested proof lists the
  SLD fact/rule steps and their provenance, so a `.adj` program that emits
  observations plus `rule { ... }` can show how the derived premise licensed the
  probabilistic verdict.

## [0.9.0] - 2026-06-21 — render corroborating citations (ADJ-A9)

### Added

- Each clause's provenance JSON now carries a **`"corroborations"`** array —
  `[{ "source": …, "locator": … }]` — listing the co-equal citations attached to
  the clause via `cites … locator …`. Empty in the common single-citation case.
  Existing `"source"/"locator"/"trust"` fields are unchanged, so existing
  recall/proof consumers keep working.

## [0.8.0] - 2026-06-17 — governing answers carry their grounded `context` (ADJ73 PR-B-3)

### Added

- Each `governing` answer now carries a **`"context"`** field — the grounded context its
  highest-standing deriving rule is in (`ninth_circuit`, `district_court`, `federal`, `state`, …),
  omitted for a context-free derivation. The audit reader sees *which context governed*, not just
  which term beat another — the lex-superior story made legible in the output.

### Worked example (committed, runnable)

- `code/specs/data/context-precedence/` — a grounded context-precedence rulebook
  (`context-precedence.adj`: `outranks_context` edges, each byte-quoting its charter — the
  Supremacy Clause for `federal > state`, vertical stare decisis for `ninth_circuit >
  district_court`) + a worked legal example (`worked-legal-example.adj`) that `import`s it and
  proves *lex superior* end-to-end: the Ninth Circuit's broad reading **governs** a district
  court's narrow reading **despite the latter's higher `mandatory` tier**. `SOURCES.md` is the
  provenance ledger. New golden test `tests/context_precedence_e2e.rs` runs the committed
  artifacts through the built CLI and asserts the override + that the edge is recallable WITH its
  charter. 0 answer-time model calls.

## [0.7.0] - 2026-06-16 — `governing` section: defeasible precedence output (ADJ73 PR-3)

### Added

- **`"governing"`** output section, emitted for each `$variable` binding query alongside
  `"recall"`. Runs `logic_engine::enumerate_governing` and renders every distinct answer with
  its precedence verdict: `status` ∈ `governing` | `defeated` (+ `defeated_by`) | `conflict_peer`,
  plus its `standing` (`asserted` for a fact, else the rule's tier) and `bindings` + ground
  `term`. A top-level `has_conflict` flags an unresolved tie. 0 answer-time model calls.
- For a predicate **not** declared `functional`, every answer is `governing` (mirrors `recall`)
  — the precedence-resolved view is back-compatible. For a functional predicate with
  `priority:` tiers it shows the override chain (the Python runtime reads this to get the
  governing decision — the enabler for the MYCIN `decide_timing` → ADJ refactor).

### Tests

- `tests/governing_e2e.rs`: the section tags every binding answer (non-functional baseline);
  it is absent for a ground hypothesis query. The functional-override path is covered at the
  engine (`logic-engine::govern`) + adj-lang lowering levels (the `functional`/`priority:`
  surface lands in adj-lang PR-C).

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
