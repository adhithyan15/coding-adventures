# LANG18 — VM Coverage

**Layer:** `vm-core` (data collection) + `tetrad-runtime` (source-line projection)
**Depends on:** LANG01 (IIR), LANG02 (vm-core), LANG06 (debug sidecar), LANG13 (DebugSidecar)
**Status:** Implemented

---

## 1  Overview

LANG18 adds lightweight, opt-in code-coverage collection to the LANG pipeline.

The feature has two layers:

1. **`vm-core` coverage mode** — the dispatch loop records which IIR instruction
   indices have been executed, keyed by function name.  The overhead is a single
   boolean guard per instruction (identical to the LANG06 debug-mode gate), and
   is zero when coverage mode is off.

2. **`tetrad-runtime` line coverage** — combines the IIR coverage data with the
   `DebugSidecar` built by LANG06's `sidecar_builder` to produce a
   `LineCoverageReport`: a mapping from source file → covered source lines →
   IIR instruction count per line.

Together they answer "which source lines of my Tetrad program were executed?"

---

## 2  vm-core coverage API

### 2.1  State added to `VMCore`

```python
# LANG18 coverage state — zero cost when coverage mode is off.
self._coverage_mode: bool = False
self._coverage: dict[str, set[int]] = {}   # fn_name → set of IIR IPs executed
```

### 2.2  Dispatch-loop integration

Immediately after `ip_before = frame.ip` is captured (and after the LANG06
debug-mode check), the dispatch loop adds:

```python
# LANG18: record instruction execution for coverage (one boolean check overhead).
if vm._coverage_mode:
    if frame.fn.name not in vm._coverage:
        vm._coverage[frame.fn.name] = set()
    vm._coverage[frame.fn.name].add(ip_before)
```

This is placed **before** `_dispatch_one`, so an instruction that raises an
exception is still counted as "executed" (it was reached).

### 2.3  Public API

| Method | Signature | Description |
|--------|-----------|-------------|
| `enable_coverage()` | `() -> None` | Enter coverage mode. |
| `disable_coverage()` | `() -> None` | Exit coverage mode (data is preserved). |
| `is_coverage_mode()` | `() -> bool` | True when coverage is active. |
| `coverage_data()` | `() -> dict[str, frozenset[int]]` | Snapshot of executed IIR indices per function (immutable copy). |
| `reset_coverage()` | `() -> None` | Clear all coverage data and disable coverage mode. |

`coverage_data()` returns `frozenset` values so callers cannot accidentally
mutate the live coverage sets.

### 2.4  Coverage and debug mode

Coverage mode and debug mode are **independent**.  Both can be active
simultaneously.  The dispatch loop checks `_coverage_mode` and `_debug_mode`
in separate if-blocks.

### 2.5  Coverage and JIT handlers

When a JIT handler is registered for a function, the interpreter path is
bypassed — no frame is pushed and `_coverage` is not updated for that
function.  This is the correct behaviour: JIT-compiled code runs natively
and would require a different instrumentation mechanism.  `is_debug_mode()`
should still be checked before registering JIT handlers when coverage is
active, as is done for LANG06 debugging.

---

## 3  tetrad-runtime `LineCoverageReport`

### 3.1  Data model

```python
@dataclass
class CoveredLine:
    """One source line that was reached during execution."""
    file: str          # Source file path (as stored in the sidecar)
    line: int          # 1-based source line number
    iir_hit_count: int # Number of distinct IIR instruction indices at this line
                       # that were executed

@dataclass
class LineCoverageReport:
    """Source-line coverage produced by composing IIR coverage with DebugSidecar."""
    covered_lines: list[CoveredLine]
    # All unique (file, line) pairs that were reached.

    def lines_for_file(self, path: str) -> list[int]:
        """Return sorted list of covered line numbers for ``path``."""

    def total_lines_covered(self) -> int:
        """Total number of distinct (file, line) pairs that were reached."""
```

### 3.2  `TetradRuntime.run_with_coverage`

```python
def run_with_coverage(
    self,
    source: str,
    source_path: str,
) -> LineCoverageReport:
    """Compile and run ``source`` with coverage enabled.

    Returns a ``LineCoverageReport`` that maps source lines to the set of
    covered lines.  The IIR coverage data is composed with the DebugSidecar
    (built internally) to project back to source lines.
    """
```

Internally:

1. Call `compile_with_debug(source, source_path)` to get `(module, sidecar)`.
2. Create a fresh `VMCore`, call `vm.enable_coverage()`.
3. Run `vm.execute(module, ...)`.
4. Read `vm.coverage_data()` and `DebugSidecarReader(sidecar)`.
5. For every `(fn_name, ip_set)` in coverage data:
   - For every `ip` in `ip_set`: call `reader.lookup(fn_name, ip)`.
   - If `lookup` returns a `SourceLocation`, accumulate `(file, line)`.
6. Build and return a `LineCoverageReport`.

### 3.3  Example

```python
rt = TetradRuntime()
report = rt.run_with_coverage(
    source="""
        x := 10
        if x > 5:
            y := x * 2
        end
    """,
    source_path="myprogram.tetrad",
)
print(report.lines_for_file("myprogram.tetrad"))
# → [2, 3, 4]   (lines that were actually executed)
```

---

## 4  Design decisions

### D1  IIR-level granularity, not Tetrad-bytecode-level

We collect coverage at the IIR instruction level, not the Tetrad bytecode
level.  This is the right level because:

- The IIR is what the VM actually executes.
- Multiple IIR instructions may correspond to one Tetrad bytecode, so IIR-level
  data is strictly more detailed.
- Projection back to source lines is done by `DebugSidecarReader.lookup`, which
  already handles the IIR → source-location mapping.

### D2  `set[int]` not `dict[int, int]` (count per IP)

Coverage only records *whether* an instruction was reached, not *how many
times*.  "Which lines were covered?" is the primary question; hit counts per
line can be derived from the `iir_hit_count` in `CoveredLine`, which counts
how many *distinct* IIR instruction indices mapping to that source line were
reached — not execution frequency.  Frequency tracking would require a
`dict[int, int]` counter and is left as a future extension.

### D3  No branch coverage

Branch coverage (tracking which branch arms were taken) is the domain of
LANG17's `BranchStats`.  LANG18 is line-coverage only.

### D4  Zero cost when disabled

The `_coverage_mode` boolean guard costs one comparison per instruction, the
same pattern used by `_debug_mode` for LANG06.  When coverage is off, no
memory is allocated and no instructions are counted.

---

## 5  Non-goals

- Source-level branch coverage (see LANG17)
- Function/statement coverage distinct from line coverage
- Coverage merging across multiple runs
- HTML / LCOV report generation (this is a data layer; reporting is a tool concern)
