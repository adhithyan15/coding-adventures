# Changelog

## [0.1.0] - 2026-06-11 — linear-equality solving (ADJ constraints track B2a)

### Added

- Initial crate: the first solver behind the adj-lang constraint sublanguage
  (track B1). `solve(&ConstraintSystem) -> SolveOutcome` handles the
  **linear-equality** case — a square system of `=` constraints over the
  declared symbols.
- Translates each constraint's unevaluated `ComputeExpr` trees into
  `symbolic-ir` `Equal` equations (symbols → `Symbol`, literals → exact
  `Integer`/`Rational`, `+`/`-`/scalar-`×`/division-by-constant → the linear
  forms cas-solve understands) and dispatches to
  `cas_solve::solve_linear_system` (exact Gaussian elimination over ℚ).
- `SolveOutcome`:
  - `Solved { assignments, from_constraints }` — a unique solution, each value
    cited to the constraints that determined it (provenance).
  - `NoUniqueSolution` — singular / non-square.
  - `Unsupported { reason }` — outside this slice (inequalities, a non-linear
    term like `x*y`, an aggregation, no symbols). **Never a wrong answer.**
- 8 unit tests (2-var system, single equation, decimal coefficients, non-square
  → no-unique, inequality/non-linear/no-symbol → unsupported, num_to_ir).

### Scope

Linear equalities only. Inequality / linear-real feasibility (QF_LRA, C1),
optimization (simplex, C2), boolean/SAT + linear-integer (`constraint-engine`),
the UNSAT-core infeasibility certificate, and CLI wiring are the next slices —
all over the same `ConstraintSystem` input.
