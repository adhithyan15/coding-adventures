# Changelog

## [0.15.0] - 2026-07-23 — absorb `ComputeExpr::ToPercent` (NUM-6c)

### Changed

- The solver's `ComputeExpr` walkers gain a `ComputeExpr::ToPercent` arm (the new NUM-6c
  `to_percent` rendering): it renders a number to a boundary string, so the LIA/linear/CAS
  extractors return `None` (out of scope, like the round/to_scientific family), the
  observed-value substitution rebuilds it with its operand substituted, and
  `is_constant_expr` recurses into its operand. No behaviour change for existing inputs —
  cross-producer totality for the new engine variant.

## [0.14.0] - 2026-07-22 — absorb `ComputeExpr::ToScientific` (NUM-6c)

### Changed

- The solver's `ComputeExpr` walkers gain a `ComputeExpr::ToScientific` arm (the new
  NUM-6c `to_scientific` rendering): it renders a number to a boundary string, so the
  LIA/linear/CAS extractors return `None` (out of scope, exactly like a round), the
  observed-value substitution rebuilds it with its operand substituted, and
  `is_constant_expr` recurses into its operand. No behaviour change for existing
  inputs — this is cross-producer totality for the new engine variant.

## [0.13.0] - 2026-07-22 — absorb `ComputeExpr::Round` (NUM-6a)

### Changed

- The solver's `ComputeExpr` walkers gain a `ComputeExpr::Round` arm (the new
  NUM-6a `round_to` narrowing): non-linear/non-polynomial, so the LIA/linear/CAS
  extractors return `None` (out of scope, like the unary round family), the
  observed-value substitution rebuilds it with its operand substituted, and
  `is_constant_expr` recurses into its operand. No behaviour change for existing
  inputs — this is cross-producer totality for the new engine variant.

## [0.12.0] - 2026-07-01 — absorb `ComputeExpr::Unary` (`ComputeOp::Abs`)

### Changed

- The solver's `ComputeExpr` walkers (LIA predicate extraction, linear-form
  extraction, observed-value substitution, polynomial extraction, the CAS-IR
  bridge, and the constant-expr check) now handle the new
  `ComputeExpr::Unary(ComputeOp, …)` from logic-engine 0.28.0. An absolute value
  is **piecewise-linear**, not affine/polynomial, so every arithmetic-solving
  path returns `None` for it (the substitution walker recurses to substitute
  inside the `|…|`, and the constant-expr check treats `|c|` as constant iff its
  operand is) — a `|x|` constraint stays `Unknown`, never silently mis-solved.
  solver 0.11 → 0.12.

## [0.11.0] - 2026-07-01 — polynomial path reads `ComputeOp::Pow`

### Changed

- `poly_of` (the univariate-polynomial recogniser behind the nonlinear root
  tactic) now understands `ComputeOp::Pow(base, n)` for a **constant
  non-negative integer** exponent `n`: it folds `base` into a polynomial `n`
  times (`base^0 = 1`). This keeps `constrain latex "$x^2 = 4$"` solving as a
  quadratic (→ {±2}) — and `x^3`, `x^4` as cubics/quartics — now that the
  adj-lang latex adapter lowers `x^n` to a native `Pow` node rather than an
  `x*x*…` expansion (adj-lang 0.20.0). A symbolic or fractional exponent is
  (as before) not polynomial → the constraint is treated as non-linear.
- `n` is bounded by `MAX_POLY_POW` (64) so a pathological `x^{huge}` cannot
  balloon the coefficient vector; the univariate solvers cover only degree ≤ 4,
  so the cap loses nothing real.

## [0.10.0] - 2026-06-14 — general boolean-clause recognizer (n-ary combinations scale)

### Added / Changed

- **The SAT set-cover path now accepts ANY single-clause boolean constraint**, not
  just at-least-one covering (`Σ xᵢ ≥ 1`). A new `classify_clause` recognizes a
  `{−1,+1}`-coefficient constraint as a clause iff, after normalizing to `≥`, the
  bound equals `1 − |negatives|` (it excludes exactly one assignment). This covers:
  at-least-one covering, **the two implications of an AND-linearization**
  (`¬y ∨ dᵢ` and `y ∨ ¬d₁ … ∨ ¬dₖ`), and `{0,1}` bounds; a true cardinality
  constraint (`≥ 2 of …`) is still deferred to LIA.
- **Why it matters:** an **n-ary combination** (a requirement covered only by a
  *subset* of selected elements — e.g. vancomycin + ceftriaxone covering resistant
  pneumococcus, which neither covers alone) is modeled by an aux boolean `y = AND(…)`,
  whose two defining constraints are exactly those implication clauses. Before, a
  combination-laden cover hit one of those constraints, failed the narrow `Σx ≥ 1`
  match, and fell back to the LIA enumeration (the ~24-selector ceiling). Now the
  whole combination cover stays on the **scalable SAT path**, so combinations scale
  to a full formulary — and each k-element combination is just k+1 clauses, linear
  in k. Defeasance (a covering edge voided by an observed fact) is an emit-time
  concern (the defeated element is dropped from the clause) and needs no engine change.
- +2 tests: a 3-element combination cover (all three selected) routes through SAT;
  a combination is **not** paid for when a single agent already covers the
  requirement.

### Unchanged

- Behavior-preserving for every prior case (all 54 earlier tests pass; the existing
  covering, fractional-relaxation, scale, uncoverable, and scalar tests are
  unchanged). `b = rk − lk` now uses `checked_sub` (defers to LIA on overflow rather
  than wrapping). General-integer / `maximize` / `: scalar` paths untouched.

## [0.9.0] - 2026-06-14 — SAT-scaled set-cover (a full-formulary feasibility oracle)

### Added

- **A SAT-based feasibility oracle for pure-boolean minimum-cost set-cover**, so
  the regimen optimization scales from ~24 selectors (the LIA enumeration ceiling)
  to a **full hospital formulary (100+ candidate drugs)**. When a `minimize` is
  over all-boolean symbols and every constraint is an at-least-one covering clause
  (`Σ xᵢ ≥ 1`) or a trivial `{0,1}` bound, `optimize` routes the binary-search-on-
  cost feasibility probes to the DPLL `SatTactic` instead of `LiaTactic`. The
  optimum is **identical** to the LIA path (verified by tests) — only the oracle
  changes — because the covering structure is exactly what SAT unit-propagation
  exploits.
- The cost bound `Σ wᵢ·xᵢ ≤ K` at each probe is encoded with a **Sinz (2005)
  sequential at-most-k counter** (each tier weight `wᵢ` modeled by repeating `xᵢ`
  `wᵢ` times; aux vars namespaced `__pb…`, uncollidable with user symbols). The
  encoder is **verified exact against brute force** over all assignments on small
  weighted instances.
- +3 tests: the Sinz encoder vs brute force (incl. weighted), SAT agrees with LIA
  on the fractional case (optimum 2, not 1.5), and a 30-selector cover the LIA
  enumeration could not finish.

### Performance (honest)

- Realistic formulary structure (a few broad drugs + many narrow): 33 drugs 0.6 s,
  63 drugs 3 s, 123 drugs 15 s, 243 drugs ~3 min — all **correct**, where the LIA
  path could not run past ~24 at all. A real per-patient candidate set (~10–30
  drugs) is sub-second.
- The tactic is plain DPLL (no clause learning), so an **adversarial** worst case
  (a pure cycle cover with no broad drug) still slows past ~30–50. A CDCL/PB-native
  solver would lift that; the encoding + oracle interface are already in place for
  that swap.

### Unchanged

- General-integer programs, `maximize`, and `: scalar` optimization take the
  existing LIA / Fourier–Motzkin paths byte-for-byte — the SAT path is gated to the
  pure-boolean clausal set-cover shape and falls through otherwise. All prior tests
  pass (the existing set-cover tests now exercise the SAT path and return the same
  answers).

## [0.8.0] - 2026-06-13 — integer linear optimization (native minimum-cost set-cover)

### Added

- **`optimize` now solves integer programs exactly** when every declared symbol is
  integer- or boolean-sorted (`: int` / `: integer` / `: bool` / `: boolean`) and
  the objective + constraints are integer-linear. This makes a minimum-cost
  **set-cover** — pick the fewest/cheapest items (drugs) whose union covers every
  requirement (organism), with `x ∈ {0,1}` — a native, proof-carrying engine
  result. The real LP relaxation returns *fractional* selections (`0.5·vancomycin`),
  which is meaningless for a yes/no choice; the integer optimum is the real answer.
- Method (no new tactic — reuses the exact pieces already here): an initial
  feasible integer point from the exact `LiaTactic`; a **structural bound** for
  boolean objectives (each `x ∈ {0,1}` contributes within `[min(0,c), max(0,c)]`,
  so the objective is bracketed for *any* number of variables); then a **binary
  search** on the objective threshold `K`, each probe an exact LIA solve, so the
  returned optimum and witness are exact. Booleans are pinned to `{0,1}` as
  explicit assertions so the LIA search stays bounded (a formulary of N drugs is a
  `2^N` search the budget handles to N ≲ 21).
- The structural boolean bound is what lets set-cover **scale past the
  Fourier–Motzkin variable cap** (which tops out at a handful of variables) — an
  8-drug cover is solved instantly. General-integer objectives still use the FM
  relaxation for the bound (small systems).
- +6 tests: cheapest single agent beats three narrow ones; the integer optimum
  beats the fractional relaxation (1.5 → 2); 8-variable scale; an uncoverable
  requirement is `Infeasible`; boolean maximize; and `: scalar` optimization is
  unchanged (the integer path is gated on the declared sort, so prior behavior is
  byte-for-byte preserved).

### Unchanged

- All real-valued (`: scalar`, `: money(...)`) optimization takes the existing
  Fourier–Motzkin path exactly as before — the integer path is opt-in via sort.

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
