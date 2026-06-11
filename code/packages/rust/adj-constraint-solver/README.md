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

## Where it fits / what's next

Part of the ADJ constraint-solving arc
([design](../../../specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md)).
Still to come: **inequality / linear-real feasibility** (QF_LRA, track C1),
**linear optimization** (simplex, C2), **boolean/SAT and linear-integer** via
`constraint-engine`, and the **infeasibility (UNSAT-core)** certificate. Those
reuse the same `ConstraintSystem` input.
