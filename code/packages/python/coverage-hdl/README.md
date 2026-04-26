# coverage-hdl

Functional + toggle coverage for the silicon stack. Subscribes to HardwareVM value-change events; records hits in user-defined bins.

See [`code/specs/coverage.md`](../../../specs/coverage.md).

## Quick start

```python
from coverage_hdl import (
    CoverageRecorder, Coverpoint, CrossPoint,
    bin_value, bin_range,
)
from hardware_vm import HardwareVM

vm = HardwareVM(hir)
cov = CoverageRecorder(vm)

cov.enable_toggle_coverage(["sum", "cout"])

cov.add_coverpoint(Coverpoint(
    name="cin",
    signal="cin",
    bins=[bin_value("zero", 0), bin_value("one", 1)],
))

cov.add_coverpoint(Coverpoint(
    name="overflow",
    signal="cout",
    bins=[bin_value("yes", 1), bin_value("no", 0)],
))

# Drive stimulus...
for a in range(16):
    for b in range(16):
        for cin in (0, 1):
            vm.set_input("a", a)
            vm.set_input("b", b)
            vm.set_input("cin", cin)

print(cov.overall_coverage)   # 1.0 if every bin hit at least once
print(cov.report().toggle)
```

## v0.1.0 scope

- `Coverpoint`: watches one signal; samples produce hits in matching bins.
- `Bin` constructors: `bin_value(name, v)`, `bin_range(name, lo, hi)`, `bin_default()`.
- `CrossPoint`: cross-product of multiple coverpoints; sampled manually via `sample_cross()`.
- `enable_toggle_coverage(signals)`: counts 0->1 (rising) and 1->0 (falling) transitions.
- `CoverageReport`: per-coverpoint hits, per-cross hits, per-signal toggle stats.
- `overall_coverage` property: average across all coverpoints + crosses.

## Out of scope (v0.2.0)

- Code coverage (line/branch/path) — needs HIR provenance instrumentation.
- FSM-state and FSM-transition coverage helpers.
- HTML reports.
- MC/DC analysis.
- Coverage merging across multiple test runs.

## Testing

```bash
pytest tests/
ruff check src/
```

MIT.
