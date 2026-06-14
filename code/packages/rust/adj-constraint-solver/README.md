# adj-constraint-solver

The solver behind the adj-lang **constraint sublanguage**. The model extracts a
policy's unknowns and constraints (`symbol` / `constrain` / `solve for`); this
crate solves them deterministically on the CPU and returns the answer **traced
to the constraints that determined it** — so the result is auditable and the
model never solved anything.

## What it does (track B2a)

Takes an `adj_lang::ConstraintSystem` and solves its **linear-equality** core —
a square system of `=` constraints over the declared symbols — via
[`cas-solve`](../cas-solve)'s exact Gaussian elimination over the rationals (no
float drift). It translates each constraint's unevaluated `ComputeExpr` trees
into `symbolic-ir` equations, dispatches to `solve_linear_system`, and parses
the `Rule(var, value)` results back into `(name, value)` assignments.

```rust
use adj_lang::compile;
use adj_constraint_solver::{solve, SolveOutcome};

let lowered = compile(
    "symbol x : scalar\n\
     symbol y : scalar\n\
     constrain x + y = 10\n\
     constrain x - y = 2\n\
     solve for { x, y }\n",
).unwrap();

match solve(&lowered.constraints) {
    SolveOutcome::Solved { assignments, from_constraints } => {
        // x = 6, y = 4 ; from_constraints cites which constraints fixed them
    }
    _ => {}
}
```

## Outcomes

- `Solved { assignments, from_constraints }` — a unique solution, each value
  cited to the constraints that determined it (provenance).
- `NoUniqueSolution` — singular / non-square (≠ one equation per unknown).
- `Unsupported { reason }` — outside this slice (an inequality, a non-linear
  term like `x*y`, an aggregation, or no symbols). **Never a wrong answer** —
  the caller falls back to a richer solver.

## Feasibility — `check` (track B2c)

A `check` request asks the dual question: is the whole constraint set *jointly
satisfiable*? `check(&cs, &kb)` translates the linear (in)equalities to
`constraint-core` `Predicate`s and runs `constraint-engine`'s `LiaTactic`
(linear integer arithmetic), returning a [`FeasibilityOutcome`]:

- `Sat { assignments }` — an **integer** witness per symbol (`x >= 3 ; x <= 5`
  → e.g. `x = 3`).
- `Unsat { core }` — a **minimal** infeasible subset (IIS): the irreducible set
  of constraints in conflict (removing any one makes the rest feasible). The
  machine-checked "*these* clauses contradict" — `x >= 5 ; x <= 3` → core
  `[0, 1]`, and an irrelevant third constraint is excluded.
- `Unknown { reason }` — a `!=` (disjunctive) or a non-linear constraint.
  **Never a false verdict.**

Observed facts are substituted first (shared with the `solve` path), so a
mixed symbol/observed system is decided with the observed values pinned.

### Real feasibility — QF_LRA via Fourier–Motzkin (track C1)

`check` decides **real** feasibility, not just integer. The linear-integer
tactic above runs first; a **Fourier–Motzkin elimination over ℚ** takes over
when that tactic punts, when a constraint is non-integer, **or when it reports
`Unsat`** — because an integer-infeasible system may still be real-feasible:

- `SatReal { assignments }` — a **rational** witness (rendered as `f64`) proving
  real satisfiability (`2x = 1` → `x = 0.5`; `0.25 <= x <= 0.75` → an interior
  point). The feasibility *decision* is exact; the witness is a representative
  point, re-checked before it is returned.
- A system is `Unsat` only when *both* the integer and real layers reject it.

Two guards bound the classic Fourier–Motzkin blow-up (and keep the i64-backed
rationals clear of overflow): caps on intermediate-inequality count and
coefficient magnitude; past either, `check` returns `Unknown`.

### Optimization — `minimize` / `maximize` (track C2)

An `objective` declares a linear program. `optimize(&cs, &kb)` maximizes (or
minimizes) it subject to the `constrain` half-planes, over exact rationals, by
**Fourier–Motzkin projection** (reusing the feasibility machinery — *not* a
separate simplex): bound a fresh variable `z` by the objective (`z ≤ obj`),
project out every decision variable, and read `z`'s least upper bound as the
optimum.

- `Optimal { value, assignments, binding }` — the optimal value, an achieving
  point, and the constraint indices **binding** (tight) at the optimum (the
  provenance of the bound). E.g. `max 3x + 2y` s.t. `x+y≤4, x≤3, x,y≥0` → `11`
  at `(3, 1)`, binding `[x+y≤4, x≤3]`.
- `Unbounded` — the objective has no bound in the feasible region.
- `Infeasible { core }` — the constraints have no feasible point; `core` is the
  minimal infeasible subset (IIS).
- `Unknown { reason }` — a non-linear/`!=` constraint or objective, or an **open
  supremum** (a strict inequality prevents the optimum being attained).

## Integer optimization (native minimum-cost set-cover)

When **every** declared symbol is integer- or boolean-sorted (`: int` /
`: integer` / `: bool` / `: boolean`) and the objective + constraints are
integer-linear, `optimize` solves the **integer program** exactly instead of the
real LP. This is what makes a minimum-cost **set-cover** — pick the
fewest/cheapest items (drugs) whose union covers every requirement (organism),
each selector `x ∈ {0,1}` — a native, proof-carrying engine result. The real LP
relaxation returns *fractional* selectors (`0.5·vancomycin`), meaningless for a
yes/no choice; the integer optimum is the real answer.

```
symbol vancomycin  : bool        % choose this drug, or not
symbol ceftriaxone : bool
constrain vancomycin + ceftriaxone >= 1   % cover the organism (here, jointly)
minimize 1 * vancomycin + 1 * ceftriaxone % fewest / lowest preference-cost
```

Method (reusing the exact pieces already here, no new tactic): the exact
`LiaTactic` gives an initial feasible integer point; a **structural bound** for
boolean objectives (each `x ∈ {0,1}` contributes within `[min(0,c), max(0,c)]`,
so the objective is bracketed for *any* number of variables — this is what lets
it scale past the Fourier–Motzkin variable cap); then a **binary search** on the
objective threshold, each probe an exact LIA solve, so the optimum and witness are
exact. Booleans are pinned to `{0,1}` so the search stays bounded (`2^N`, fine to
N ≲ 21). The integer path is **opt-in via the declared sort** — `: scalar` /
`: money(...)` programs take the real Fourier–Motzkin path exactly as before.

### Scaling set-cover — the SAT oracle

For a **pure-boolean** minimum-cost set-cover (all selectors `: bool`, every
constraint reducible to a single CNF clause), `optimize` routes the binary-search
feasibility probes to the DPLL `SatTactic` instead of LIA's bounded enumeration.
The cost bound `Σ wᵢ·xᵢ ≤ K` at each probe is a **Sinz sequential at-most-k**
encoding (verified exact vs brute force). Same optimum as the LIA path; the
difference is reach: LIA tops out around **24 selectors**, the SAT oracle handles
a **full hospital formulary (100+ candidate drugs)** — 123 drugs in ~15 s, where
LIA could not run at all. (Plain DPLL, so an adversarial cycle cover still slows
past ~30–50; a real per-patient candidate set of ~10–30 drugs is sub-second. A
CDCL/PB-native solver would lift the worst case — the oracle interface is already
in place for that swap.)

The clause recognizer (`classify_clause`) accepts **any** `{−1,+1}`-coefficient
constraint that is a single clause — at-least-one covering (`Σ xᵢ ≥ 1`), `{0,1}`
bounds, and the two implications of an **AND-linearization** (`¬y ∨ dᵢ`,
`y ∨ ¬d₁ … ∨ ¬dₖ`). That last one is what makes an **n-ary combination** — a
requirement satisfied only by a *subset* of selected elements (`y = AND(d₁…dₖ)`) —
stay on the scalable SAT path; each k-element combination is just k+1 clauses,
linear in k. A genuine cardinality constraint (`≥ 2 of …`) is still deferred to LIA.

## Where it fits

Part of the ADJ constraint-solving arc
([design](../../../specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md)).
The dimensional + constraint stack (solve / feasibility / optimization) is now
complete; remaining work is the nonlinear long tail and worked-example
integration. Reuses the same `ConstraintSystem` input throughout.
