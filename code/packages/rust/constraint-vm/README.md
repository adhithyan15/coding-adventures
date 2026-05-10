# `constraint-vm`

**LANG24 PR 24-D** — the Constraint-VM instruction-stream executor.

Walks a `Program` one opcode at a time, drives the `Engine`, and returns a
`VmOutput` describing what the program produced.  The "easy crate" in the
LANG24 stack — all the interesting solving is in `constraint-engine`.

---

## Architecture

```
constraint-instructions — Program (instruction stream)
        │
constraint-vm  (this crate)
        │   dispatches each opcode; maintains scope stack (Engine snapshots)
        │
constraint-engine — Engine::check_sat() → SolverResult
```

## Scope stack and incremental solving

`PushScope` saves an engine snapshot.  `PopScope` restores it, undoing all
assertions since the push.  This lets consumers (like `lang-refinement-checker`)
explore alternative hypotheses without rebuilding the solver from scratch.

## Resource limits

| Limit | Default |
|-------|---------|
| Max instructions per run | 10 000 |
| Max scope depth | 100 |
| Max assertions total | 10 000 |

Exceeding any limit returns `VmError::LimitExceeded` rather than running
forever or panicking.

## Quick start

### Using `ProgramBuilder`

```rust
use constraint_vm::ProgramBuilder;
use constraint_core::{Sort, Logic};

let prog = ProgramBuilder::new()
    .set_logic(Logic::QF_LIA)
    .declare_int("x")
    .assert_ge_int("x", 0)
    .assert_le_int("x", 100)
    .check_sat()
    .get_model()
    .build();

let model = constraint_vm::get_model(&prog).unwrap().unwrap();
let x = model.get("x").unwrap().as_int().unwrap();
assert!(x >= 0 && x <= 100);
```

### Using the convenience functions

```rust
use constraint_vm::{check_sat, get_model, ProgramBuilder};
use constraint_core::Logic;

// Just check satisfiability.
let prog = ProgramBuilder::new()
    .set_logic(Logic::QF_LIA)
    .declare_int("x")
    .assert_ge_int("x", 5)
    .assert_le_int("x", 3)   // contradiction
    .check_sat()
    .build();

let result = check_sat(&prog).unwrap();
assert!(result.is_unsat());
```

## Relationship to LANG23

`lang-refinement-checker` uses this crate as its backend.  The pattern is:

1. Build a constraint program encoding the proof obligation `E ∧ ¬P`.
2. Call `check_sat()`.
3. `Unsat` → `PROVEN_SAFE`; `Sat(model)` → `PROVEN_UNSAFE`; `Unknown` → emit runtime check.
