# Changelog — `cdcl-sat`

## 0.3.1 — 2026-11-09

Audit release. No behaviour change: the audit found no soundness, proof or
completeness bug in the solver (see the contracts C1–C9 in VER09 §6.1).

### Added

- `tests/audit.rs` (28 tests), measured against oracles that share no code
  with the solver (brute force, model re-evaluation, `drat-check`):
  - exhaustive domains: 4 495 formulas of at most 3 clauses of width at most 2
    over 3 variables (empty, repeated-literal and tautological clauses
    included), 27 405 formulas of at most 4 clauses of width at most 3 over 3
    variables, 6 545 formulas of at most 3 clauses of width at most 2 over 4
    variables, and an assumption domain of 254 475 solves (every formula of at
    most 2 clauses, every assumption set of at most 3 literals);
  - every UNSAT answer is checked by `drat-check`, with and without the
    deletion steps; every failed-assumption core is checked against brute
    force and certified by `drat-check` (core as units plus the solver's proof
    plus the empty clause);
  - seeded random 3-SAT, n = 5..20, ratio 3.8–4.8; long-lived solvers
    answering many assumption queries; incremental clause batches;
  - tiny conflict and work budgets, resumed to completion, never give a wrong
    answer and any proof recorded across `Unknown` still checks; same input
    gives identical answers, stats and proof text;
  - pigeonhole PHP(n+1, n) n ≤ 6 (certified), XOR cycles and chains, Tseitin
    on K3..K6;
  - clause-database reduction on 175-vertex formulas with certified proofs;
  - malformed DIMACS, limits, and a differential check that the solver's and
    the kernel's DIMACS readers accept and reject the same texts;
  - a search-statistics fingerprint pinning VSIDS decay, clause activity,
    restarts and reduction (correctness tests cannot see a heuristic that is
    silently switched off).
- VER09 §6.1: testable contracts C1–C9; §6 rewritten to what is actually
  logged; divergences 7–12 recorded.
- `set_work_budget` documentation: resuming with a constant small work budget
  can livelock; grow the budget.

## 0.3.0 — 2026-11-02

### Added

- `Solver::set_work_budget` and `Stats::work` (watch-list visits during
  propagation). A conflict budget alone does not bound time on formulas
  with millions of clauses; the work budget does. When it runs out, `solve`
  returns `Unknown` and the solver stays usable. Added for the VER10 BV
  tactic after its security review timed a 155-byte 512-bit multiplication
  query at more than 300 s.

## 0.2.0 — 2026-11-02

VER11: DRAT proof logging.

### Added

- `Solver::enable_proof` (before the first clause), `proof`, `take_proof`,
  and `Solver::from_cnf_with_proof`. The proof is recorded in memory as a
  `DratProof` of `ProofStep`s; `DratProof::to_drat_text` renders DRAT text.
- Logged events (VER11 §4): clauses shortened or emptied by level-0
  simplification in `add_clause`, every learned clause, every deletion made
  by database reduction, and the empty clause on a level-0 conflict.
  Original clauses are not logged; they are the formula.
- `drat-check` as a dev-dependency only. `tests/drat.rs` certifies every
  UNSAT answer of the differential corpus (600 random formulas and
  pigeonhole up to PHP(8,7)). A negative corpus is rejected in full:
  truncated, reordered and tampered proofs, deletions of absent clauses,
  and proofs replayed against satisfiable weakenings of their formulas.
- `tests/common/` shares the generator, brute force and pigeonhole helpers
  between the integration tests.

## 0.1.0 — 2026-11-02

Initial release: the VER09 CDCL SAT solver.

### Added

- `Var`, `Lit` (packed `2·var + sign`) and three-valued `LBool`, with DIMACS
  conversion.
- `Solver`: two-watched-literal propagation with blocker literals, first-UIP
  conflict analysis with recursive clause minimisation, non-chronological
  backjumping, VSIDS with an activity heap, phase saving, Luby restarts,
  and learned-clause reduction by LBD and activity. Freed clause slots are
  reused.
- Solving under assumptions, with `failed_assumptions()` cores, and adding
  clauses between solves. Learned clauses persist across calls.
- A conflict budget: `SolveResult::Unknown` when it runs out, and the solver
  stays usable.
- Strict DIMACS CNF reader (`parse_dimacs`, `parse_dimacs_with_limit`) with
  line-numbered errors and a variable-count cap (default 2^20, about
  100 MB of solver state; caller limits clamp to the 2^31 encoding
  ceiling), plus a writer
  (`write_dimacs`) and `Cnf::is_satisfied_by` for checking models.
- Differential tests against brute force (plain, under assumptions,
  incremental), pigeonhole instances, and DIMACS round trips.
