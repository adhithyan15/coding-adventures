# Constraint-VM Enhancement — Optimization & Scheduling-Scale Solving

> Part of the [task-app spec series](task-app-overview.md), but the work lands in the **existing**
> `constraint-*` crates, not in task-app. It fulfils the standing instruction to *"fix the constraint
> VM so it supports all the features."* The task-app scheduler
> ([`task-app-scheduling-engine.md`](task-app-scheduling-engine.md)) is the first consumer, but these
> are general SMT/OMT capabilities that benefit every user of the stack.

## Current state (audited)

The `constraint-*` stack is a clean, SMT-LIB-style decision procedure:

- `constraint-core` — `Predicate` AST (linear arithmetic), `Sort` (`Bool`/`Int`/`Real`/…),
  `Logic` (`QF_Bool`, `QF_LIA`, `QF_LRA` declared, `free_vars`, `infer_sort`, NNF/CNF/simplify).
- `constraint-instructions` — `ConstraintInstr` opcodes (`DeclareVar`, `Assert`, `CheckSat`,
  `GetModel`, `GetUnsatCore`, `PushScope`/`PopScope`, `Reset`, `SetLogic`) + text parser/serializer.
- `constraint-engine` — `Engine::check_sat() -> SolverResult::{Sat(Model), Unsat, Unknown(String)}`;
  `Model` maps names to `Value::{Int(i128), Bool(bool)}`; a boolean CDCL core (`sat.rs`) + a
  linear-integer tactic (`lia.rs`).
- `constraint-vm` — `Vm` + `ProgramBuilder` fluent API (`set_logic`, `declare_int`, `assert_pred`,
  `assert_ge_int/le_int/eq_int`, `check_sat`, `get_model`, `push_scope`/`pop_scope`, `build`) and
  free functions `check_sat(&Program)` / `get_model(&Program)`.

**Two gaps** make it unable to *drive* scheduling optimization today:

1. **No objective / optimization.** The engine is satisfiability-only — `SolverResult` has no
   `Optimal`, and there is no `Minimize`/`Maximize` opcode. `get_model` returns an *arbitrary*
   feasible point (typically variables pushed to a lower bound), not a cost-minimal one. Resource
   leveling and makespan minimization are inexpressible.
2. **The LIA tactic is a bounded-search heuristic.** `lia.rs` collects per-variable bounds, picks a
   witness, substitutes, and recurses — its own docs warn it is "potentially exponential" and may
   return `Unknown` on many-variable coupled systems. A real scheduling network (dozens–hundreds of
   interacting `start`/`finish` variables) is exactly that case.

`adj-lang` already *parses* `minimize`/`maximize` (its `OptDir`), and `adj-constraint-solver` names
LP/simplex as an unimplemented "track C2" — so the front-end vocabulary exists but the solver side is
empty. We implement it.

## Goals

Add, without breaking existing satisfiability behavior:

1. **Optimization Modulo Theories (OMT)** — minimize/maximize a linear objective, returning a
   provably optimal model.
2. **A difference-logic fast path** that solves the scheduling fragment in polynomial time and
   detects infeasibility (negative cycles) exactly — making the engine robust at scheduling scale.
3. The plumbing to expose both through instructions, the VM, the builder, and `adj-lang`.

## Change 1 — Objective representation (`constraint-core`)

Add a linear **objective term** type (reusing the arithmetic sub-AST already inside `Predicate`):

```rust
/// A linear term: Σ coefficient·variable + constant. (Same shape as the arithmetic already
/// embedded in Predicate::{Add,Sub,Mul,Var,Int}; extracted here so objectives and constraints share it.)
pub struct LinearTerm { pub terms: Vec<(i128, String)>, pub constant: i128 }

pub enum Objective { Minimize(LinearTerm), Maximize(LinearTerm) }
```

`LinearTerm::free_vars`, `Display` (SMT-LIB s-expr), and a `from_predicate_arith` helper keep it
consistent with the existing normalization/printing conventions.

## Change 2 — Opcodes (`constraint-instructions`)

Add two opcodes mirroring SMT-LIB `(minimize t)` / `(maximize t)`, plus `(get-objectives)`:

```rust
pub enum ConstraintInstr {
    // …existing…
    Minimize { term: LinearTerm },
    Maximize { term: LinearTerm },
    GetObjectives,          // after CheckSat: report objective value(s) of the optimal model
}
```

Extend the text parser/serializer (`minimize`, `maximize`, `get-objectives`) and the
`check_program` validator (objective vars must be declared; objectives require an optimizing
`CheckSat`). Programs with no objective behave exactly as before (`CheckSat` = satisfiability).

## Change 3 — The OMT loop (`constraint-engine`)

Add an optimization result and an `optimize` entry point layered over the existing `check_sat`:

```rust
pub enum OptResult {
    Optimal { model: Model, value: i128 },
    Unbounded,                 // objective decreases without bound
    Unsat,
    Unknown(String),
}
impl Engine {
    pub fn optimize(&mut self, obj: &Objective) -> OptResult { /* … */ }
}
```

**Algorithm — bound-refinement OMT over the existing theory solver** (correct for `QF_LIA`, which is
what scheduling uses):

1. `check_sat()`. If `Unsat`/`Unknown`, return the same. Else take the model `m₀`, objective value `v₀`.
2. Push a scope; assert `obj < v₀` (for minimize). `check_sat()` again.
3. If `Sat(m₁)` with value `v₁ < v₀`: keep `m₁`, tighten again (linear descent; optionally
   **binary search** between a known feasible `v` and a lower bound derived from the constraints to
   cut iterations to `O(log range)`).
4. When the tightened query is `Unsat`, the last feasible model is **optimal** → `Optimal{m, v}`.
5. Unbounded detection: if no lower bound exists and each tighten stays `Sat`, return `Unbounded`
   (guarded by the engine's instruction/assertion resource limits).

This reuses `push_scope`/`pop_scope` (already stale-invalidate the cached result — see
`constraint-vm` line ~336) so each refinement is incremental. It is exact for integer objectives and
needs no new theory — it is a driver around `check_sat`.

## Change 4 — Difference-logic tactic for scheduling scale (`constraint-engine/lia.rs`)

Scheduling constraints are almost all **difference constraints**: `start_B − finish_A ≥ lag`,
`finish_T − start_T = duration`, `start_T ≥ project_start`. The fragment where every assertion is
`x − y ≤ c` (Integer Difference Logic, QF_IDL) is solvable in **polynomial time** via shortest paths
and detects infeasibility as a **negative cycle** — precisely what over-constrained plans produce.

Add a **difference-logic recognizer + Bellman-Ford solver** as a fast tactic ahead of the general
LIA search:

1. Normalize each asserted linear predicate; if *all* are two-variable difference constraints (plus
   simple bounds `x ≤ c` via a virtual zero node), build the constraint graph (edge `y →(c) x` for
   `x − y ≤ c`).
2. Run Bellman-Ford from the zero node. A **negative cycle ⇒ `Unsat`** (with the cycle as an
   unsat-core hint). Otherwise the shortest-path distances are a **feasible model** (and, for
   scheduling, the tightest — matching CPM early dates when minimizing starts).
3. If any assertion falls outside the fragment, fall back to the existing (now clearly-scoped) LIA
   search. Combined with OMT bound-refinement, minimizing makespan over a pure-DL network is then
   polynomial per iteration.

This makes the engine **robust and fast on exactly the networks task-app produces**, and its
negative-cycle detection is the feasibility check the scheduler surfaces to the UI.

## Change 5 — VM & builder surface (`constraint-vm`)

Extend `ProgramBuilder` and add optimizing entry points, keeping the satisfiability API untouched:

```rust
impl ProgramBuilder {
    pub fn minimize(self, term: LinearTerm) -> Self;   // pushes Minimize
    pub fn maximize(self, term: LinearTerm) -> Self;
    pub fn get_objectives(self) -> Self;
}
/// Free function mirroring check_sat/get_model.
pub fn optimize(program: &Program) -> Result<OptResult, VmError>;
```

`VmOutput` gains `last_opt_result()`. `adj-constraint-solver` routes `adj-lang`'s already-parsed
`OptDir::{Minimize,Maximize}` to `optimize` instead of erroring, closing its "track C2".

## Backward compatibility & safety

- Every existing program (no objective) produces byte-identical results — the OMT loop and DL tactic
  are only entered when an `Objective` is present or the pure-DL fragment is detected; the general
  `check_sat` path is otherwise unchanged.
- Existing resource limits (default 10k instructions / 10k assertions) bound the OMT loop; add an
  explicit `max_optimize_iterations` to `Config`.
- `#![forbid(unsafe_code)]` remains; no new `unsafe`.

## Testing

- **OMT correctness**: minimize/maximize on small linear programs with hand-computed optima; unbounded
  and infeasible cases; binary-search vs. linear-descent agree on the optimum.
- **Difference logic**: feasible networks return shortest-path (CPM-early) models; negative-cycle
  networks return `Unsat` with the cycle; mixed fragments fall back correctly.
- **Scheduling-scale**: generated CPM networks (100–500 tasks) solve within limits where the old LIA
  path returned `Unknown` — the regression the enhancement targets.
- **No-regression**: the entire existing `constraint-*` test suite passes unchanged; a property test
  asserts objective-free programs give identical `SolverResult` before/after.
- Update each touched crate's CHANGELOG and README; add worked OMT + difference-logic examples.
