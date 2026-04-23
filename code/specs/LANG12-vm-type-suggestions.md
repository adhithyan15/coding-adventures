# LANG12 — vm-type-suggestions: Parameter Type Suggestions from the VM Profiler

## Overview

When a developer writes an untyped function:

```python
def add(a, b):
    return a + b
```

and runs it 1,000,000 times where `a` and `b` are always integers, the VM
has observed something the developer hasn't said: *these parameters are
always numbers*.  The developer could annotate them as `Int`, and the
compiler could optimise from the very first call — no warmup, no JIT
speculation, no guards.

`vm-type-suggestions` surfaces this observation in the simplest possible
form:

```
add — called 1,000,000 times
  parameter 'a': always u8  → declare 'a: u8'
  parameter 'b': always u8  → declare 'b: u8'
```

This is distinct from `jit-profiling-insights` (LANG11), which analyzes
JIT guard and deoptimisation overhead *after* compilation.  `vm-type-
suggestions` works purely at the interpreter tier — no JIT required — and
answers one focused question:

> "Based on what the VM actually observed, what type annotations should I add?"

---

## The suggestion loop

```
1. Developer writes a program with untyped parameters.

2. Program runs under vm-core's interpreter for N calls.
   The VMProfiler records observed_type on every load_mem [arg[N]]
   instruction (the instructions that load function arguments).

3. vm-type-suggestions reads those observations and emits:

   Function: add — 1,000,000 calls
     'a' (arg 0): always u8  → annotate as 'u8'
     'b' (arg 1): always u8  → annotate as 'u8'

   Function: format_value — 3 calls
     's' (arg 0): polymorphic (u8 + str)  → cannot suggest; mixed types observed

4. Developer adds the annotations, reruns.  On the next run,
   the compiler sees typed parameters from call 1, emits optimised
   code with no guards needed.
```

---

## Data model

### `Confidence`

```python
class Confidence(str, Enum):
    CERTAIN   = "certain"    # One concrete type, 100% of observations
    MIXED     = "mixed"      # Multiple types observed (polymorphic)
    NO_DATA   = "no_data"    # Profiler never reached this parameter
```

### `ParamSuggestion`

```python
@dataclass
class ParamSuggestion:
    function: str           # Function name
    param_name: str         # Parameter name from IIRFunction.params
    param_index: int        # 0-based argument position
    observed_type: str      # "u8", "str", … or "polymorphic" or None
    call_count: int         # How many times this param was observed
    confidence: Confidence  # CERTAIN / MIXED / NO_DATA
    suggestion: str | None  # "declare 'n: u8'" or None if no suggestion
```

### `SuggestionReport`

```python
@dataclass
class SuggestionReport:
    program_name: str
    total_calls: int                     # Sum of call counts across all functions
    suggestions: list[ParamSuggestion]   # All params, including NO_DATA / MIXED

    def actionable(self) -> list[ParamSuggestion]:
        """Return only CERTAIN suggestions — the ones to act on."""
        ...

    def by_function(self) -> dict[str, list[ParamSuggestion]]:
        """Group suggestions by function name."""
        ...

    def format_text(self) -> str: ...
    def format_json(self) -> str: ...
```

---

## The suggestion algorithm

### Step 1 — Identify untyped parameters

For each `IIRFunction`, find all parameters whose `type_hint == "any"`.
Already-typed parameters need no suggestion.

### Step 2 — Find the profiler observation for each parameter

In the IIR, function arguments are loaded via `load_mem [arg[N]]`
instructions at the top of the function body.  `vm-core`'s profiler fills
`observed_type` and `observation_count` on these instructions as the
function runs.

Match each untyped parameter to its `load_mem [arg[N]]` instruction by
index:

```
param index 0  →  load_mem instr with srcs[0] == "arg[0]"
param index 1  →  load_mem instr with srcs[0] == "arg[1]"
...
```

### Step 3 — Classify the observation

```
if instr not found or instr.observation_count == 0:
    → NO_DATA   (parameter was never observed)

elif instr.observed_type == "polymorphic":
    → MIXED     (multiple types seen — no safe suggestion)

else:
    → CERTAIN   (one concrete type observed on every call)
```

### Step 4 — Generate suggestion text

```
CERTAIN   → "declare '{param}: {observed_type}'"
MIXED     → None  (cannot suggest; inform the developer of the mixed types)
NO_DATA   → None  (no data; function may not have been called)
```

---

## Public API

```python
from vm_type_suggestions import suggest, SuggestionReport, ParamSuggestion

# fn_list: list[IIRFunction] from a post-run IIRModule
report: SuggestionReport = suggest(fn_list, program_name="my_program")

# All actionable suggestions (CERTAIN confidence only)
for s in report.actionable():
    print(f"  {s.function}.{s.param_name}: always {s.observed_type} "
          f"({s.call_count:,} calls) → {s.suggestion}")

# Human-readable output
print(report.format_text())

# JSON for tooling (LSP inline hints, editor tooltips)
print(report.format_json())
```

---

## Example output

```
VM Type Suggestions — fibonacci (1,048,579 total observations)
══════════════════════════════════════════════════════════════

✅ fibonacci — 1,048,576 calls
  'n' (arg 0): always u8
  → declare 'n: u8'  [eliminates all type guards on this parameter]

✅ main — 3 calls
  'result' is a local, not a parameter — no suggestion

⚠️  format_value — 3 calls
  's' (arg 0): mixed types observed (u8 + str)
  → cannot suggest; consider two typed overloads instead

Summary: 1 of 1 untyped parameter can be annotated.
```

---

## Integration points

### vm-core integration

```python
class VMCore:
    def type_suggestions(self, program_name: str = "program") -> SuggestionReport:
        """Return parameter type suggestions from the most recent run."""
        from vm_type_suggestions import suggest
        return suggest(self._module.functions, program_name=program_name)
```

### Language tool integration

- **REPL**: After each run, print `report.actionable()` if any suggestions exist.
- **LSP**: Feed `report.format_json()` to inline parameter hints in the editor
  — the developer sees `'n: u8'` greyed out next to the parameter name.
- **CLI**: `--suggest-types` flag prints the report after execution.
- **jit-profiling-insights**: Use `vm-type-suggestions` as the first-pass tool
  to guide annotation; then use LANG11 to verify guards were eliminated.

---

## Relationship to jit-profiling-insights (LANG11)

| | vm-type-suggestions | jit-profiling-insights |
|---|---|---|
| **Works at** | Interpreter tier | JIT-compiled tier |
| **Needs JIT?** | No | Yes |
| **Question answered** | "What types should I annotate?" | "What overhead does the JIT still have?" |
| **Primary output** | `"declare 'n: u8'"` | `"GUARD: 1 branch/call"` |
| **When to use** | Before/during development | After profiling a hot path |

The two tools are complementary:
1. Run `vm-type-suggestions` → add annotations → rerun
2. Run `jit-profiling-insights` → verify guards eliminated → fix any remaining

---

## Module layout

```
vm-type-suggestions/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── vm_type_suggestions/
        ├── __init__.py
        ├── types.py     # Confidence, ParamSuggestion, SuggestionReport
        └── suggest.py   # suggest() — the main entry point
```

---

## Testing strategy

- Unit tests for `suggest()` with every `Confidence` case.
- Tests for already-typed parameters (skipped — no suggestion needed).
- Tests for functions with no `load_mem [arg[N]]` instructions (no data).
- Tests for polymorphic parameters (MIXED — no suggestion).
- Integration test mirroring the `fibonacci(n)` example.
- Golden-file tests for `format_text()` and `format_json()`.

Target: **95%+ line coverage**.
