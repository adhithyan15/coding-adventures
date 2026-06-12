# Changelog

## [0.4.0] - 2026-06-11 — feasibility / `check` via linear-integer tactic (ADJ constraints track B2c)

### Added

- **`FeasibilityOutcome` + `check(&ConstraintSystem, &KnowledgeBase)`** — a
  `check` request now decides whether the whole constraint set is jointly
  satisfiable, not just whether one variable can be solved for. The linear
  (in)equalities are translated to `constraint-core` `Predicate`s and handed to
  `constraint-engine`'s `LiaTactic` (linear integer arithmetic):
  - `Sat { assignments }` — a witness integer per symbol proving satisfiability
    (`x >= 3 ; x <= 5` → e.g. `x = 3`).
  - `Unsat { core }` — the constraint indices whose conjunction is contradictory
    (`x >= 5 ; x <= 3` → unsat).
  - `Unknown { reason }` — a constraint outside linear-integer scope (nonlinear,
    or not integer-valued) the tactic cannot accept.
- Observed facts are substituted before solving (shares `substitute_observed`
  with the `solve` path), so a `check` over a mix of symbols and observed
  values is decided with the observed values pinned.
- `relop_predicate` / `expr_to_pred` / `int_const` bridge the adj-lang
  `ComputeExpr` + `RelOp` to the `Predicate` AST. 4 new feasibility tests
  (sat witness, unsat conflict, observed substitution, nonlinear → unknown).

## [0.3.0] - 2026-06-11 — nonlinear single-unknown solving (ADJ constraints track C3)

### Added

- **`SolveOutcome::SolvedRoots { var, roots, from_constraints }`** — a single
  unknown satisfying a **nonlinear** (degree 2–4) equality is now solved for its
  real roots: `constrain x * x = 4` → `{-2, 2}`, `x² − 5x + 6 = 0` → `{2, 3}`,
  `x² = 2` → `{±√2}` (numerically), cubic `{1, 2, 3}`.
- The constraint's `lhs − rhs` is built into a univariate polynomial
  (`poly_of`/`poly_add`/`poly_mul`), its degree-2/3/4 coefficients converted to
  exact `Frac`, and solved via `cas_solve::{solve_quadratic,solve_cubic,
  solve_quartic}`. Roots are evaluated to f64 (`eval_ir_root` handles rational
  and `Sqrt` irrational forms); **complex roots are dropped** (real roots only),
  and an all-complex equation (`x² + 1 = 0`) → `Unsupported`.
- Scope: **one** unknown, degree ≤ 4. Multi-unknown nonlinear (`x*y`) and
  degree > 4 stay `Unsupported`/`NoUniqueSolution` — never a wrong answer.
  Degree ≤ 1 still goes through the exact linear path.

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
