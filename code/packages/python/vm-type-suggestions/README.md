# vm-type-suggestions

Parameter type suggestions from the VM profiler.

After running a program, this package tells you: "this function was always called with integers — annotate it so the compiler can optimise from call one."

```
add — called 1,000,000 times
  'a' (arg 0): always u8  → declare 'a: u8'
  'b' (arg 1): always u8  → declare 'b: u8'
```

No JIT required. No guard analysis. Just: "here's what came in — say so in the source code."

See [LANG12-vm-type-suggestions.md](../../../../specs/LANG12-vm-type-suggestions.md) for the full design spec.

## Quick start

```python
from vm_type_suggestions import suggest

report = suggest(module.functions, program_name="fibonacci")

# All actionable suggestions
for s in report.actionable():
    print(f"  {s.function}.{s.param_name}: {s.suggestion}")

# Human-readable output
print(report.format_text())
```

## How it works

The VM profiler (`vm-core`) already records what type each argument was on every call, via `observed_type` on `load_mem [arg[N]]` instructions. This package reads those observations and classifies each untyped parameter as:

- `CERTAIN` — always one concrete type → suggest the annotation
- `MIXED` — multiple types observed → cannot safely suggest
- `NO_DATA` — function never called → no data
