# Coverage

## Overview

Coverage measures how thoroughly a testbench exercises a design. Two complementary kinds:

1. **Code coverage** — automatic, derived from the design itself: which lines were executed? which branches were taken? which signals toggled? which FSM states reached?
2. **Functional coverage** — user-defined: which scenarios were tested? which input combinations seen? did the cross-product of `(opcode × pipeline-stage)` get fully covered?

This spec defines instrumentation hooks and reporting for both. It piggybacks on `hardware-vm.md` events; coverage is a "shadow process" that subscribes to value-change events and updates counters. After simulation, a HTML/JSON report shows hit/miss rates.

## Layer Position

```
hardware-vm.md ─── value-change events ──► coverage.md
        │                                        │
        ▼                                        ▼
 (testbench-framework.md drives the VM)   coverage report
```

## Concepts

### Code coverage kinds

| Kind | What it measures | Detected via |
|---|---|---|
| **Line coverage** | Which source lines executed | HIR provenance + execution events |
| **Branch coverage** | Which sides of each `if/case` taken | HIR if/case nodes + statement-execution events |
| **Toggle coverage** | Which signals went 0→1 and 1→0 | Subscribe to value-change events |
| **FSM-state coverage** | Which states reached | Subscribe to state-register value changes |
| **FSM-transition coverage** | Which (from-state, to-state) edges traversed | Track previous + current state |
| **Path coverage** | Which paths through nested conditionals | Combinatorial; warn on explosion |
| **Expression coverage** | Which combinations of sub-expressions affected the result | MC/DC analysis |

We implement line, branch, toggle, FSM-state, FSM-transition for v1. Path and expression as future work.

### Functional coverage

User declares **cover groups** with **cover points**:

```python
import coverage as cov

@cov.covergroup
class AluCoverage:
    op = cov.coverpoint("op", bins=[
        cov.bin("ADD", 0),
        cov.bin("SUB", 1),
        cov.bin("AND", 2),
        cov.bin("OR",  3),
    ])
    a_signed = cov.coverpoint("a", bins=[
        cov.bin_range("negative", min=0x80000000, max=0xFFFFFFFF),
        cov.bin_range("zero",     min=0,          max=0),
        cov.bin_range("positive", min=1,          max=0x7FFFFFFF),
    ])
    cross_op_a = cov.cross(op, a_signed)
```

Each `coverpoint` watches a signal; when that signal changes, the value is checked against bins; matching bin's hit-count increments. `cross` records the joint hit of multiple coverpoints.

### Sampling

Coverpoints sample either:
- **On every change** (default for value-change-driven designs).
- **On a clock edge** (for synchronous coverage).
- **At an explicit `sample()` call** (manual control).

### Reporting

After simulation:
- **HTML report**: source-line-annotated view; bins per coverpoint; cross-bin matrices; missing bins highlighted.
- **JSON report**: machine-readable for CI integration.

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum
from typing import Callable


@dataclass(frozen=True)
class Bin:
    name: str
    matcher: Callable[[int], bool]


def bin(name: str, value: int) -> Bin:
    return Bin(name, lambda v: v == value)


def bin_range(name: str, min: int, max: int) -> Bin:
    return Bin(name, lambda v: min <= v <= max)


@dataclass
class Coverpoint:
    name: str
    signal: str
    bins: list[Bin]
    sample_kind: str = "on_change"   # "on_change" | "on_edge" | "manual"
    sample_signal: str | None = None  # for "on_edge"


@dataclass
class CrossPoint:
    coverpoints: list[Coverpoint]


@dataclass
class CoverageReport:
    coverpoints: dict[str, dict[str, int]]    # coverpoint name → {bin name: hits}
    crosses: dict[str, dict[tuple[str, ...], int]]
    line_coverage: dict[str, dict[int, int]]  # file → {line: hits}
    branch_coverage: dict[str, dict[int, dict[str, int]]]
    toggle_coverage: dict[str, dict[str, int]]    # signal → {direction: hits}
    fsm_state_coverage: dict[str, dict[str, int]]
    fsm_transition_coverage: dict[str, dict[tuple[str, str], int]]


class CoverageRecorder:
    def __init__(self, vm: "HardwareVM"):
        ...
    
    def add_coverpoint(self, cp: Coverpoint) -> None: ...
    def add_cross(self, cross: CrossPoint) -> None: ...
    def enable_line_coverage(self) -> None: ...
    def enable_branch_coverage(self) -> None: ...
    def enable_toggle_coverage(self, signals: list[str]) -> None: ...
    def enable_fsm_coverage(self, fsm_signal: str, states: list[str]) -> None: ...
    
    def sample(self, coverpoint_name: str | None = None) -> None:
        """Manually sample. If coverpoint_name is None, sample all manual coverpoints."""
        ...
    
    def report(self) -> CoverageReport: ...
    def write_html(self, path: Path) -> None: ...
    def write_json(self, path: Path) -> None: ...
```

## Worked Example — 4-bit Adder coverage

```python
from coverage import CoverageRecorder, coverpoint, bin

cov = CoverageRecorder(vm)
cov.enable_toggle_coverage(["sum[0]", "sum[1]", "sum[2]", "sum[3]", "cout"])

cov.add_coverpoint(coverpoint(
    name="cin", signal="cin", bins=[bin("0", 0), bin("1", 1)]
))
cov.add_coverpoint(coverpoint(
    name="overflow", signal="cout", bins=[bin("yes", 1), bin("no", 0)]
))

# Run 256 stimulus from testbench
# After:
report = cov.report()
print(report.toggle_coverage)
# {"sum[0]": {"0->1": 128, "1->0": 128}, ...}

print(report.coverpoints["cin"])
# {"0": 128, "1": 128}

print(report.coverpoints["overflow"])
# {"yes": 16, "no": 240}    (16 input pairs cause overflow)
```

100% toggle coverage; 100% cin coverage; both overflow bins hit. Test thorough.

## Worked Example — FSM coverage

```python
cov.enable_fsm_coverage("state", states=["RED", "GREEN", "YELLOW"])

# Run testbench: 3 cycles of clk
# After:
report = cov.report()
print(report.fsm_state_coverage["state"])
# {"RED": 1, "GREEN": 1, "YELLOW": 1}

print(report.fsm_transition_coverage["state"])
# {("RED","GREEN"): 1, ("GREEN","YELLOW"): 1, ("YELLOW","RED"): 1}
```

100% state and transition coverage with just 3 cycles.

## Edge Cases

| Scenario | Handling |
|---|---|
| Value not in any bin | Record as "uncategorized"; report says "value 5 not in any bin." |
| Cross product explosion (many coverpoints crossed) | Warn if cross-product > 10000 bins. |
| Coverpoint signal doesn't exist | Compile-time error. |
| Sampling on every cycle for a 1M-cycle test | Performant; coverage events are O(1) per change. |
| HTML report for thousands of coverpoints | Pagination + filtering. |
| Coverage merging (multiple test runs) | `report_a + report_b` returns merged report. |

## Test Strategy

### Unit (95%+)
- Bin matching: each kind (point, range, default).
- Toggle counting.
- FSM transitions: detect each edge.

### Integration
- Run testbench-framework adder test with coverage; confirm 100% toggle on outputs.
- ALU test with 8 ops × 3 a-bins × 3 b-bins cross; 72 bins, full hit at 1000 random vectors.

## Conformance

| Standard | Coverage |
|---|---|
| **SystemVerilog `covergroup`** (1800-2017 §19) | Subset (no `option.weight`, no `iff` guards in v1) |
| **VHDL coverage** | No standard; we follow PSL convention loosely |
| **MC/DC** (DO-178 modified condition/decision) | Out of scope; future spec |

## Open Questions

1. **MC/DC** — relevant for safety-critical; defer.
2. **Cross-merge across regressions** — yes, supported via `report + report`.

## Future Work

- MC/DC analysis.
- Coverage-driven test generation.
- Functional coverage UI.
- Real-time coverage feedback in interactive simulation.
