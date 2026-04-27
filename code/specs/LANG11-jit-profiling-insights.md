# LANG11 — jit-profiling-insights: Developer Feedback from the JIT Compiler

## Overview

Today's JIT compilers — V8, HotSpot, PyPy, LuaJIT — make decisions about
specialization, deoptimization, and type guards *silently*.  A developer
writes:

```python
def add(a, b):
    return a + b
```

and has no idea whether the JIT compiled it to a single `ADD` instruction, or
whether it generates a type check on every call because `a` and `b` are
sometimes integers and sometimes strings.

`jit-profiling-insights` gives developers **visibility into the JIT's
decisions** and actionable advice about where adding a type annotation would
eliminate overhead.  This is the JIT counterpart to a traditional profiler:
instead of "this function took 40ms", it says "this expression causes 3 type
guards per call because `x` is declared untyped".

---

## The insight loop

```
1. Developer writes a program in a gradual-typing language (Tetrad, Nib, …)
   with some typed variables and some untyped.

2. Program runs under jit-core's interpreter + JIT for N iterations.

3. jit-core's profiler records, per instruction:
   - How many times it was executed
   - What type(s) were observed on each source register
   - Whether a type guard was emitted (speculation cost)
   - Whether a guard failure occurred (deoptimization cost)

4. jit-profiling-insights reads the profiler's IIR annotations and produces
   a structured report:

   Hot site: add in loop_body — 12,847 calls
     Observed type: int (100%)
     Current code path: generic runtime call  [expensive]
     Root cause: variable 'x' is declared without a type annotation
     Suggestion: annotate 'x' as Int
     Estimated speedup: ~15%  (eliminates 1 generic call per iteration)

5. Developer adds the annotation, reruns, sees confirmation.
```

---

## Data model

### `ObservedProfile`

The profiler in `jit-core` already annotates each `IIRInstr` with
`observed_type` and `observation_count` (from the `IIRInstr` dataclass in
`interpreter-ir`).  The insight pass reads these annotations directly from
the post-JIT `IIRFunction` — no new profiler infrastructure is needed.

### `TypeSite`

A `TypeSite` represents one instruction in one function that the insight pass
has identified as a candidate for improvement.

```python
@dataclass
class TypeSite:
    function: str
    instruction_op: str
    source_register: str       # The untyped register causing the overhead
    observed_type: str         # What the profiler actually saw (e.g. "int")
    type_hint: str             # What the source said ("any" = untyped)
    dispatch_cost: DispatchCost
    call_count: int
    deopt_count: int
    savings_description: str   # Human-readable: "would eliminate 3 guards/call"
```

### `DispatchCost`

```python
class DispatchCost(str, Enum):
    NONE = "none"               # Typed statically or inferred — no cost
    GUARD = "guard"             # One type_assert per use of this register
    GENERIC_CALL = "generic"    # Full generic runtime dispatch (worst case)
    DEOPT = "deopt"             # Guard failed; function fell back to interpreter
```

### `ProfilingReport`

The top-level output of the pass.

```python
@dataclass
class ProfilingReport:
    program_name: str
    total_instructions_executed: int
    sites: list[TypeSite]       # Sorted by impact (call_count × cost weight)

    def top_n(self, n: int = 10) -> list[TypeSite]: ...
    def functions_with_issues(self) -> list[str]: ...
    def format_text(self) -> str: ...
    def format_json(self) -> str: ...
```

---

## The insight algorithm

### Step 1 — Scan instrumented IIR

After running the program, `jit-core` has annotated each `IIRInstr` with:
- `observed_type` — the runtime type(s) seen on this instruction's destination
- `observation_count` — how many times this instruction executed

The insight pass iterates over every `IIRInstr` in every function.

### Step 2 — Classify dispatch cost

For each instruction, determine the dispatch cost:

```
if instr.type_hint != "any":
    → NONE  (statically typed — JIT compiles to a direct typed op)

elif instr.op == "type_assert":
    → GUARD  (the JIT inserted this guard because type_hint is "any"
               but inferred type is concrete)

elif instr.op == "call_runtime" and "generic_" in instr.srcs[0]:
    → GENERIC_CALL  (inferred type is also "any" — full dynamic dispatch)

elif instr.observation_count > 0 and instr.deopt_count > 0:
    → DEOPT  (a guard was emitted but failed at runtime — interpreter fallback)

else:
    → NONE
```

### Step 3 — Find the root cause register

When a `GUARD` or `GENERIC_CALL` is found, the overhead originates from
the *source register* being untyped.  The insight pass traces back along the
data-flow chain to find the register whose `type_hint == "any"` caused the
cost:

```
type_assert %r0, "int"  ← GUARD on %r0
  └─ %r0 = load_mem [arg[0]]   ← %r0's type_hint is "any" because it came from
                                   an untyped function parameter
```

The root cause is the untyped function parameter, not the `type_assert` itself.

### Step 4 — Rank by impact

Each site's impact score is:

```
impact = call_count × cost_weight

where cost_weight:
    NONE         →  0
    GUARD        →  1     (one branch per call)
    GENERIC_CALL →  10    (runtime dispatch ≈ 10× slower than typed op)
    DEOPT        →  100   (interpreter fallback ≈ 100× slower)
```

Sites are sorted descending by impact.  The developer sees the worst
offenders first.

### Step 5 — Generate advice

For each high-impact site, the insight pass generates a human-readable
message:

```
Hot site: add_u8 in fibonacci — 1,048,576 calls
  Source register: %r2 (function parameter 'n')
  Observed type: int (100% of calls)
  Current path: type_assert(%r2, int) on every call [GUARD — 1 branch/call]
  Root cause: parameter 'n' has no type annotation
  Suggestion: declare 'n: Int' in the function signature
  Estimated speedup: ~8%  (1 branch eliminated from hot path)
```

---

## Public API

```python
from jit_profiling_insights import analyze, ProfilingReport, TypeSite

# Run after jit-core has executed the program
# fn_list: list[IIRFunction] with profiler annotations
report: ProfilingReport = analyze(fn_list, program_name="fibonacci")

# Print the top-10 hotspots
print(report.format_text())

# Or get structured data for tooling
import json
print(report.format_json())

# Programmatic access
for site in report.top_n(5):
    print(f"{site.function}.{site.instruction_op}: "
          f"{site.call_count} calls, cost={site.dispatch_cost}")
```

---

## Example output

```
JIT Profiling Report — fibonacci (8,388,608 total instructions)
═══════════════════════════════════════════════════════════════

🔴 HIGH IMPACT  fibonacci::add
  Source: parameter 'n' (type_hint="any")
  Observed: int (100% of 1,048,576 calls)
  Cost: GUARD — 1 type_assert per call = 1,048,576 branches
  Fix: declare 'n: Int'
  Estimated speedup: ~8%

🔴 HIGH IMPACT  fibonacci::cmp_lt
  Source: parameter 'n' (type_hint="any")
  Observed: int (100% of 1,048,576 calls)
  Cost: GUARD — 1 type_assert per call
  Fix: declare 'n: Int'  (same fix as above — one annotation eliminates both)
  Estimated speedup: bundled in above

🟡 MEDIUM IMPACT  main::call
  Source: local variable 'result' (type_hint="any")
  Observed: int (100% of 3 calls)
  Cost: GENERIC_CALL — runtime dispatch for 'add'
  Fix: declare 'result: Int' or let the type checker infer it
  Estimated speedup: ~2%

✅ No deoptimizations occurred.

Summary: 2 annotation sites would eliminate ~10% of total overhead.
```

---

## Integration points

### jit-core integration

`jit-core.JITCore` exposes a new method:

```python
class JITCore:
    def profile_report(self, program_name: str = "program") -> ProfilingReport:
        """Return a ProfilingReport from the most recent run."""
        from jit_profiling_insights import analyze
        return analyze(self._profiled_functions, program_name=program_name)
```

The `_profiled_functions` are the `IIRFunction` objects whose instructions
have been annotated by `jit-core`'s existing profiler pass.  No new data
collection is required — the profiler already records `observed_type` and
`observation_count` on each `IIRInstr`.

### Language tool integration (LSP, REPL, compiler CLI)

The `ProfilingReport` is format-agnostic.  Consumers:

- **CLI**: `format_text()` printed to stdout after a `--profile` flag
- **LSP** (`jit-core` → language server protocol server): `format_json()`
  feeds inline diagnostics — the IDE underlines untyped variables with
  "This variable causes N type guards per call; add `: Int` to eliminate them"
- **REPL** (`jit-core` → REPL): after each `run`, print the top 3 sites
- **CI/benchmarks**: `report.top_n(1)[0].dispatch_cost == DispatchCost.DEOPT`
  can fail a performance budget test

---

## What this tells developers that no existing tool does

Today's profilers (cProfile, py-spy, perf) tell you *where time is spent* but
not *why the JIT chose to spend it there* or *what you could change*.

`jit-profiling-insights` is the **first layer between profiling and advice**:

| Tool | What it tells you |
|------|-------------------|
| `cProfile` | "fibonacci took 2.3s" |
| `py-spy` | "40% of samples in `add`" |
| `jit-profiling-insights` | "In `add`, parameter `n` has no type annotation; the JIT emits a branch on every call; annotating it as `Int` eliminates the branch and saves ~8% runtime" |

The key insight is that the JIT compiler has the information to give this
feedback *during compilation* — it knows what type it inferred, what type it
observed, and what code path it chose.  We just have to surface it.

---

## Module layout

```
jit-profiling-insights/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── jit_profiling_insights/
        ├── __init__.py
        ├── types.py        # TypeSite, DispatchCost, ProfilingReport
        ├── analyze.py      # analyze() — the main entry point
        ├── classify.py     # _classify_cost(), _find_root_register()
        ├── rank.py         # impact scoring and top_n()
        └── format.py       # format_text(), format_json()
```

---

## Testing strategy

- Unit tests for `_classify_cost` with every combination of `type_hint`,
  `observed_type`, `op`, and `deopt_count`.
- Unit tests for `_find_root_register` tracing back through load chains.
- Unit tests for `rank` — verify impact ordering for GUARD vs GENERIC_CALL
  with different call counts.
- Integration tests with a synthetic `IIRFunction` that mirrors the
  `fibonacci` example above; assert the report's top site matches
  the expected register and advice string.
- Golden-file test for `format_text()` and `format_json()`.

Target: **95%+ line coverage**.

---

## Design decisions

### Why not instrument at the bytecode level?

The `IIRInstr.observed_type` and `observation_count` fields already exist in
the `interpreter-ir` dataclass.  The JIT's hot-tier profiler already populates
them.  The insight pass is purely a *reader* of existing data — no new
instrumentation hooks needed.

### Why impact score = call_count × cost_weight rather than wall time?

We don't have a cycle counter attached to each instruction in the interpreter.
`call_count × cost_weight` is a conservative proxy: it ranks DEOPT far above
GUARD, and GUARD above NONE, which matches the actual performance ordering of
these paths.  Real cycle counts from `perf` or `instruments` can always be
correlated by the developer.

### Why surface this as a library (not a CLI)?

Surfacing it as `analyze(fn_list) → ProfilingReport` keeps the insight logic
separate from any specific UI.  The CLI, LSP, and REPL all build on top of the
same structured report.  This follows the same layered design as the rest of
the LANG stack.

### Why "estimated speedup" rather than a precise figure?

A precise speedup figure would require knowing the cost of a branch vs. the
cost of the surrounding computation, which varies by CPU microarchitecture and
surrounding context.  "~8%" is a conservative estimate based on the fraction
of total instructions that are guards.  The note is useful for prioritization
("should I bother?") not for benchmarking.
