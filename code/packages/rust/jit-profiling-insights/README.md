# jit-profiling-insights

> **LANG11** — JIT dispatch-overhead profiler.  Reads the runtime observations
> recorded by `vm-core`'s profiler and ranks every hot instruction by how much
> dispatch overhead it is imposing.

---

## Where it fits in the LANG pipeline

```
Tetrad source
  → interpreter-ir (LANG01)   — IIR data model
  → vm-core       (LANG02)    — interpreter + profiler
  → jit-core      (LANG03)    — JIT specialiser
  → jit-profiling-insights    ← YOU ARE HERE (LANG11)
     "Here are the instructions causing the most overhead — fix these first."
```

---

## What it does

After running a program under `vm-core`, every `IIRInstr` carries two
profiler annotations:

| Field | Meaning |
|---|---|
| `observation_count: u32` | How many times this instruction executed |
| `observed_type: Option<String>` | The last type the profiler saw |
| `deopt_anchor: Option<usize>` | Present iff the JIT deoptimised here |

`jit-profiling-insights` assigns a **dispatch cost** to each instruction,
computes an **impact score** (`call_count × cost_weight`), and returns a
ranked `ProfilingReport` so you know exactly which instructions to
specialise first.

---

## Dispatch cost tiers

| Tier | Weight | Meaning |
|---|---|---|
| `None` | 0 | Statically typed — already optimal |
| `Guard` | 1 | One `type_assert` check per call |
| `GenericCall` | 10 | Dispatches through a `generic_*` helper |
| `Deopt` | 100 | JIT bailed out to interpreter — worst case |

A `GenericCall` at 10,000 calls/s has the same impact score as a `Deopt`
at 1,000 calls/s.

---

## Quick start

```rust
use jit_profiling_insights::{analyze, DispatchCost};
use interpreter_ir::{IIRFunction, IIRInstr, Operand};

// Build a function with one hot type_assert instruction.
let mut guard = IIRInstr::new(
    "type_assert",
    Some("r0".into()),
    vec![Operand::Var("arg[0]".into()), Operand::Lit("u8".into())],
    "any",
);
guard.observation_count = 100_000;

let fn_ = IIRFunction::new("hot_fn", vec![], "any", vec![guard]);

// Analyse with a minimum call-count threshold.
let report = analyze(&[fn_], "my_program", 1_000);

println!("{}", report.format_text());
// → TypeSite { fn: "hot_fn", call_count: 100000, cost: Guard, impact: 100000 }

assert!(report.has_deopts() == false);
assert_eq!(report.top_n(1)[0].cost, DispatchCost::Guard);
```

---

## API

### `analyze(fn_list, program_name, min_call_count) -> ProfilingReport`

Main entry point.  Scans every instruction in every function, classifies
its dispatch cost, filters out sites below `min_call_count`, ranks by
impact, and returns a `ProfilingReport`.

### `ProfilingReport`

| Method | Description |
|---|---|
| `top_n(n)` | Top N highest-impact `TypeSite`s |
| `functions_with_issues()` | Deduplicated function names with overhead |
| `has_deopts()` | `true` if any `Deopt`-tier sites exist |
| `format_text()` | Human-readable ASCII report |
| `format_json()` | JSON string for tooling |

### `TypeSite`

```rust
pub struct TypeSite {
    pub function:            String,
    pub instr_index:         usize,
    pub op:                  String,
    pub cost:                DispatchCost,
    pub call_count:          u64,
    pub deopt_count:         u64,
    pub root_var:            Option<String>,
    pub savings_description: String,
}
```

`impact() -> u64` returns `call_count × cost.weight()`.

---

## Dependencies

- [`interpreter-ir`](../interpreter-ir) — `IIRFunction` / `IIRInstr` / `Operand`

No other runtime dependencies.

---

## Running tests

```sh
cargo test -p jit-profiling-insights
```
