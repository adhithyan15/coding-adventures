# Changelog

## [0.2.0] - 2026-06-11 — observed-value substitution (ADJ constraints track B3)

### Changed

- **`solve(&ConstraintSystem, &KnowledgeBase)`** — now takes the program's KB.
  A constraint reference that is **not** an unknown but **is** an observed fact
  (`observe base_rate(1200)`) is substituted by its value before solving, so a
  realistic mixed program solves:
  `symbol premium; constrain premium = base_rate + 300; solve for {premium}`
  → `premium = 1500`. (Previously every reference was treated as an unknown, so
  any constraint mentioning an observed fact was singular.) Unknowns and
  unobserved references are left symbolic. 2 new tests.

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
