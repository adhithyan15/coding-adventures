# vm-type-suggestions

> **LANG12** — Parameter type suggester.  Reads the runtime observations
> recorded by `vm-core`'s profiler and recommends concrete type annotations
> for every untyped function parameter.

---

## Where it fits in the LANG pipeline

```
Tetrad source
  → interpreter-ir (LANG01)   — IIR data model
  → vm-core       (LANG02)    — interpreter + profiler
  → vm-type-suggestions       ← YOU ARE HERE (LANG12)
     "Based on what the VM observed, you should annotate 'n: u8'."
```

No JIT required.  No guard analysis.  Just: "here's what came in — you
should say so in the source code."

---

## What it does

After running a program under `vm-core`, every `IIRInstr` that loads a
function argument carries:

| Field | Meaning |
|---|---|
| `observation_count: u32` | How many times this instruction executed |
| `observed_type: Option<String>` | The type seen at runtime |

`vm-type-suggestions` reads the `load_mem arg[N]` instruction for each
untyped parameter, classifies the observation as `Certain` / `Mixed` /
`NoData`, and returns a `SuggestionReport` with ready-to-use annotation
strings.

---

## Confidence levels

| Level | Meaning |
|---|---|
| `Certain` | All observed calls used the same type — safe to annotate |
| `Mixed` | VM observed `"polymorphic"` — annotation would over-specialise |
| `NoData` | No calls observed, or no `load_mem` found — cannot recommend |

---

## Quick start

```rust
use vm_type_suggestions::{suggest, Confidence};
use interpreter_ir::{IIRFunction, IIRInstr, Operand};

// A function with one untyped parameter observed 1 million times as u8.
let mut load = IIRInstr::new(
    "load_mem",
    Some("r0".into()),
    vec![Operand::Var("arg[0]".into())],
    "any",
);
load.observation_count = 1_000_000;
load.observed_type = Some("u8".into());

let fn_ = IIRFunction::new(
    "add",
    vec![("n".into(), "any".into())],
    "any",
    vec![load],
);

let report = suggest(&[fn_], "fibonacci");
assert_eq!(report.suggestions[0].confidence, Confidence::Certain);
assert_eq!(
    report.suggestions[0].suggestion.as_deref(),
    Some("declare 'n: u8'"),
);

println!("{}", report.format_text());
```

---

## API

### `suggest(fn_list, program_name) -> SuggestionReport`

Main entry point.  For each function in `fn_list`, skips already-typed
parameters (`type_hint != "any"`), finds the `load_mem arg[N]` instruction
for each untyped parameter, classifies the observation, and returns a
`SuggestionReport`.

### `SuggestionReport`

| Method | Description |
|---|---|
| `actionable()` | Iterator over `Certain` suggestions only |
| `by_function()` | Suggestions grouped by function, in insertion order |
| `format_text()` | Human-readable ASCII report |
| `format_json()` | JSON string for tooling |

### `ParamSuggestion`

```rust
pub struct ParamSuggestion {
    pub function:      String,
    pub param_name:    String,
    pub param_index:   usize,
    pub observed_type: Option<String>,
    pub call_count:    u64,
    pub confidence:    Confidence,
    pub suggestion:    Option<String>,   // "declare 'param: type'" or None
}
```

---

## How the `load_mem arg[N]` convention works

IIR produced by gradual-typing language compilers loads each function
argument into an SSA register at the very start of the function body:

```text
load_mem %r0 <- arg[0] : any
load_mem %r1 <- arg[1] : any
```

`vm-core`'s profiler calls `instr.record_observation(rt)` after each
instruction that produces a value.  After N invocations of `add(a, b)`,
the `load_mem arg[0]` instruction has `observed_type = Some("u8")` and
`observation_count = N` — exactly what this crate reads.

---

## Dependencies

- [`interpreter-ir`](../interpreter-ir) — `IIRFunction` / `IIRInstr` / `Operand`

No other runtime dependencies.

---

## Running tests

```sh
cargo test -p vm-type-suggestions
```
