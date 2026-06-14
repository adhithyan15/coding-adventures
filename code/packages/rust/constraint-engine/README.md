# `constraint-engine`

**LANG24 PR 24-C** — pluggable solver tactics for the generic Constraint-VM.

The solver backend.  Pure algorithms, no I/O, no VM state — just "here is a
set of asserted predicates, is there a satisfying model?"

---

## Architecture

```
constraint-core   (predicate AST + normalisation)
      │
constraint-engine  (this crate)
      │   ├─ sat_tactic  — boolean DPLL/CDCL
      │   ├─ lia_tactic  — Cooper's algorithm for linear integer arithmetic
      │   └─ Engine      — dispatches to the right tactic; Nelson-Oppen
      │                    combination for multi-theory queries
      │
constraint-vm     (instruction-stream executor; drives Engine)
```

## Solver result

Every `check_sat()` call returns a `SolverResult`:

| Variant | Meaning |
|---------|---------|
| `Sat(model)` | Satisfiable; model maps each variable to a value |
| `Unsat` | Unsatisfiable |
| `Unknown(reason)` | Solver gave up (unsupported theory, quantifiers, …) |

This three-way split maps directly onto LANG23's three outcomes:
`PROVEN_SAFE` (Unsat), `PROVEN_UNSAFE` (Sat), `UNKNOWN → runtime check`.

## Theories supported in v1

| Logic | Tactic | Complete? |
|-------|--------|-----------|
| `QF_Bool` | SAT (DPLL) | Yes |
| `QF_LIA` | LIA (Cooper) | Yes |
| Others | — | No → Unknown |

## Usage

```rust
use constraint_core::{Sort, Predicate, Logic};
use constraint_engine::Engine;

let mut e = Engine::new();
e.set_logic(Logic::QF_LIA);
e.declare_var("x".into(), Sort::Int);
e.assert(Predicate::Ge(
    Box::new(Predicate::Var("x".into())),
    Box::new(Predicate::Int(0)),
));
e.assert(Predicate::Le(
    Box::new(Predicate::Var("x".into())),
    Box::new(Predicate::Int(100)),
));

let result = e.check_sat();
assert!(result.is_sat());
let model = result.model().unwrap();
let x = model.get("x").unwrap().as_int().unwrap();
assert!(x >= 0 && x <= 100);
```

## LIA tactic — Cooper algorithm

Implements a bounded Cooper quantifier-elimination for linear integer
arithmetic.  Variable elimination proceeds by:

1. Pick one variable.
2. Extract lower/upper bounds from all constraints.
3. Enumerate candidates in `[lo, lo + MAX_SEARCH_WIDTH]`.
4. For each candidate, substitute and recurse on the remaining formula.

Complete for the formulae arising from LANG23 refinement predicates
(bounded ranges, equalities, disequalities, linear sums).

## SAT tactic — DPLL

Standard DPLL with unit propagation and pure-literal elimination.  Used for
pure boolean formulas.  The engine automatically selects this tactic when no
integer variables are declared.
