# jit-profiling-insights

Developer feedback from the JIT compiler: visibility into type guards, generic
dispatch, and deoptimisations — with actionable type-annotation suggestions.

Today's JIT compilers (V8, HotSpot, PyPy, LuaJIT) make decisions about
specialisation, deoptimisation, and type guards *silently*.  This package
surfaces those decisions as structured, ranked advice:

| Tool | What it tells you |
|------|-------------------|
| `cProfile` | "fibonacci took 2.3s" |
| `py-spy` | "40% of samples in `add`" |
| `jit-profiling-insights` | "In `add`, parameter `n` has no type annotation; the JIT emits a branch on every call; annotating it as `Int` eliminates the branch and saves ~8% runtime" |

## Where it fits in the stack

```
IIRFunction (with profiler annotations from vm-core + jit-core)
        │
        ▼
  jit-profiling-insights.analyze()
        │
        ▼
  ProfilingReport
  ┌─────┴──────────┬────────────────┐
  ▼                ▼                ▼
format_text()  format_json()   top_n(n)
(CLI / REPL)   (LSP / CI)     (programmatic)
```

The package is a pure *reader* — it reads the `observed_type` and
`observation_count` annotations that `vm-core` and `jit-core` already write
onto each `IIRInstr`.  No new instrumentation hooks are needed.

## Quick start

```python
from jit_profiling_insights import analyze, ProfilingReport, TypeSite

# fn_list: list[IIRFunction] from a post-JIT IIRModule
report: ProfilingReport = analyze(fn_list, program_name="fibonacci")

# Human-readable terminal output
print(report.format_text())

# JSON for tooling (LSP diagnostics, CI performance gates)
import json
data = json.loads(report.format_json())

# Programmatic access — top 5 hotspots
for site in report.top_n(5):
    print(f"{site.function}.{site.instruction_op}: "
          f"{site.call_count:,} calls, cost={site.dispatch_cost}")
```

## Example output

```
JIT Profiling Report — fibonacci (8,388,608 total instructions)
═══════════════════════════════════════════════════════════════

🔴 HIGH IMPACT  fibonacci::type_assert
  Source: arg[0] (type_hint="any")
  Observed: int on 1,048,576 calls (12% of total)
  Cost: GUARD — would eliminate 1 type_assert per call (1,048,576 branches total)
  Estimated speedup: ~12%

🟡 MEDIUM IMPACT  main::call_runtime
  Source: %r0 (type_hint="any")
  Observed: int on 3 calls (0% of total)
  Cost: GENERIC — eliminates generic dispatch

✅ No deoptimisations occurred.

Summary: 2 annotation sites would eliminate ~12% of total overhead.
```

## API reference

### `analyze(fn_list, *, program_name, min_call_count) → ProfilingReport`

The main entry point.  Scans every `IIRInstr` in every `IIRFunction`, classifies
the dispatch cost, traces the data-flow chain to find the root untyped register,
and returns a ranked `ProfilingReport`.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `fn_list` | `list[IIRFunction]` | required | Post-JIT functions with profiler annotations |
| `program_name` | `str` | `"program"` | Label for the report |
| `min_call_count` | `int` | `1` | Skip instructions seen fewer than this many times |

### `DispatchCost`

Four-level enum (also a `str` mixin for direct JSON serialisation):

| Value | Weight | Meaning |
|-------|--------|---------|
| `NONE` | 0 | Statically typed — no overhead |
| `GUARD` | 1 | One `type_assert` branch per call |
| `GENERIC_CALL` | 10 | Full runtime dispatch (~10× slower) |
| `DEOPT` | 100 | Guard failed; interpreter fallback (~100× slower) |

### `TypeSite`

One instruction-level hotspot.  Key fields:

| Field | Type | Description |
|-------|------|-------------|
| `function` | `str` | Function name |
| `instruction_op` | `str` | Instruction mnemonic |
| `source_register` | `str` | Root untyped register (traced back through load chains) |
| `observed_type` | `str` | What the profiler saw (`"int"`, `"polymorphic"`, …) |
| `dispatch_cost` | `DispatchCost` | Classified overhead |
| `call_count` | `int` | Execution count from `observation_count` |
| `deopt_count` | `int` | Guard failure count (0 if no guard) |
| `impact` | `int` | `call_count × cost_weight` — the ranking key |

### `ProfilingReport`

| Method | Returns | Description |
|--------|---------|-------------|
| `top_n(n=10)` | `list[TypeSite]` | Top *n* sites by impact |
| `functions_with_issues()` | `list[str]` | Deduplicated function names with overhead |
| `has_deopts()` | `bool` | Any `DEOPT`-level sites present? |
| `format_text()` | `str` | Human-readable terminal output |
| `format_json()` | `str` | JSON for tooling |

## Integration with jit-core

```python
class JITCore:
    def profile_report(self, program_name: str = "program") -> ProfilingReport:
        from jit_profiling_insights import analyze
        return analyze(self._profiled_functions, program_name=program_name)
```

## Integration patterns

- **CLI**: Print `report.format_text()` after a `--profile` flag
- **LSP**: Feed `report.format_json()` to inline diagnostics ("This variable causes N type guards per call")
- **REPL**: Print `report.top_n(3)` after each `run`
- **CI performance gates**: `report.top_n(1)[0].dispatch_cost == DispatchCost.DEOPT` fails a budget test

## Installation

```bash
pip install coding-adventures-jit-profiling-insights
```

Requires `coding-adventures-interpreter-ir`.
