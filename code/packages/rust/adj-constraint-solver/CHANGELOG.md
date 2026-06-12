# Changelog

## [0.7.0] - 2026-06-12 — minimal infeasibility certificate (IIS) (ADJ constraints track E1)

### Changed

- **`Unsat { core }` / `Infeasible { core }` now carry a MINIMAL infeasible
  subset (an IIS)** — the irreducible set of constraints in conflict, such that
  removing any one makes the rest feasible — instead of the full constraint set.
  This is the machine-checked "*these* clauses contradict", the certificate that
  localizes a golden-rulebook bug to the exact conflicting constraints.
- Computed by a **deletion filter** (`minimal_unsat_core`): walk the constraints;
  if dropping one leaves a still-infeasible set, it is redundant and removed
  permanently — O(n) feasibility checks, each reusing the existing two-layer
  decider (`subset_is_unsat`: exact integer LIA, then real Fourier–Motzkin).
  Conservative on `Unknown` subsets (keeps the constraint), so the core is always
  a valid infeasible set; for the linear systems here it is exactly minimal.
- `check` and `optimize` both emit the minimal core. +5 tests (irrelevant
  constraints excluded, an all-needed 3-way conflict, a lone self-contradiction,
  an infeasible LP core).

## [0.6.0] - 2026-06-11 — linear optimization (`minimize`/`maximize`) via FM projection (ADJ constraints track C2)

### Added

- **`OptimizeOutcome` + `optimize(&ConstraintSystem, &KnowledgeBase)`** — solves
  the LP declared by an `objective` against the `constrain` half-planes, over
  exact rationals. Outcomes: `Optimal { value, assignments, binding }`,
  `Unbounded`, `Infeasible { core }`, `Unknown { reason }`.
- **Method:** Fourier–Motzkin **projection**, reusing the C1 machinery — not a
  new simplex. Introduce a variable `z` bounded by the objective (`z ≤ obj`),
  project out every original variable, and read `z`'s least upper bound as the
  optimum. No upper bound ⇒ `Unbounded`; a violated *constant* in the projection
  ⇒ `Infeasible`. `minimize obj = −maximize(−obj)`. The achieving assignment is
  recovered by pinning `obj = OPT` and running the C1 witness reconstruction; the
  `binding` constraints are the originals tight at the optimum.
- A **strict** tightest bound (open supremum, not attained) is reported as
  `Unknown` rather than a fake optimum. Observed facts are substituted first.
- Refactored the FM elimination loop into a shared `eliminate(planes, to_elim)`
  helper (feasibility eliminates *all* variables; optimization eliminates all
  *but* `z`). All overflow still routes to `Unknown` via the checked `Rat`.
- 9 new tests (single-var max/min, the 3x+2y=11 vertex LP, unbounded, infeasible,
  open supremum, nonlinear objective, observed substitution, two-var min).

## [0.5.0] - 2026-06-11 — QF_LRA real feasibility via Fourier–Motzkin (ADJ constraints track C1)

### Added

- **`FeasibilityOutcome::SatReal { assignments: Vec<(String, f64)> }`** — `check`
  now decides **real** (QF_LRA) feasibility, not just integer. A
  **Fourier–Motzkin elimination** over ℚ decides whether a conjunction of linear
  (in)equalities is satisfiable over the reals and reconstructs a rational
  witness (rendered as `f64`). The arithmetic uses a self-contained **checked
  i128 rational** (`Rat`) — every operation returns `None` on overflow instead
  of silently wrapping, so an overflow becomes `Unknown`, never a wrong verdict.
- `check` now **layers two procedures**: the exact linear-integer tactic
  (`LiaTactic`, B2c) runs first; the Fourier–Motzkin layer takes over when the
  integer tactic punts (`Unknown`), when a constraint is non-integer, **or when
  the integer tactic reports `Unsat`** — because an integer-infeasible system may
  still be real-feasible (`2x = 1` → `x = 0.5`). A system is `Unsat` only when
  *both* layers reject it.
- New internal machinery: `LinForm` (affine form over ℚ), `linearize`,
  `constraint_to_halfplanes` (equality → two `≤`, strict `<`/`>` tracked), the
  `fourier_motzkin` driver, and witness back-substitution with a defensive
  re-check (`witness_satisfies`) that downgrades to a witness-free `SatReal`
  rather than ever emit a wrong point.
- 9 new tests (fractional feasible/infeasible, integer-infeasible-but-real,
  two-variable sat/unsat, strict inequalities, `!=`/nonlinear → `Unknown`).

### Changed

- **BREAKING (enum):** the prior B2c behavior where a non-integer constraint
  (e.g. `x <= 0.5`) returned `Unknown` is superseded — it is now decided as
  real-feasible (`SatReal`). Callers matching on `FeasibilityOutcome` must add a
  `SatReal` arm.

### Safety

- The rational arithmetic is **checked end-to-end**: `Rat`'s ops return `None`
  on i128 overflow (or past a `RAT_CAP` magnitude ceiling that also keeps the
  ordering cross-products within i128), and every Fourier–Motzkin step —
  elimination, witness back-substitution, and the witness re-check — propagates
  that to `Unknown` (or drops the witness) rather than emit a wrapped, possibly
  sign-flipped value. A `MAX_INEQUALITIES` cap bounds the classic FM blow-up.
  (Hardens the overflow path flagged in security review: a fixed-width rational
  would wrap silently and could flip a feasibility verdict.)

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
