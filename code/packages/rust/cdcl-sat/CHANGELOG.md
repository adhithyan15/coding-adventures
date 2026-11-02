# Changelog — `cdcl-sat`

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
