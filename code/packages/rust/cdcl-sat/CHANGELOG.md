# Changelog — `cdcl-sat`

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
  line-numbered errors and a variable-count cap, plus a writer
  (`write_dimacs`) and `Cnf::is_satisfied_by` for checking models.
- Differential tests against brute force (plain, under assumptions,
  incremental), pigeonhole instances, and DIMACS round trips.
