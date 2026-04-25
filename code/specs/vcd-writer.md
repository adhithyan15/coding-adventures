# VCD Writer

## Overview

VCD (Value Change Dump) is the canonical waveform format for digital simulation, defined in IEEE 1364-2005 §18. Every simulator emits VCD; every waveform viewer (GTKWave, surfer, ModelSim) reads it. This spec defines a streaming VCD writer that subscribes to value-change events from `hardware-vm.md` and produces a `.vcd` file readable by GTKWave.

The format is text-based, append-only, and naturally streams: a value change at time t adds one line. No global rewrites, no buffering of entire simulation.

## Layer Position

```
hardware-vm.md (emits value-change events)
       │
       ▼
vcd-writer.md  ◀── THIS SPEC
       │
       ▼
.vcd file → GTKWave / surfer / etc.
```

## Format

```
$date Tue Apr 25 10:30:00 2026 $end
$version Silicon-Stack VCD Writer v0.1 $end
$comment 4-bit adder simulation $end
$timescale 1ps $end
$scope module top $end
$scope module u_dut $end
$var wire 4 ! a [3:0] $end
$var wire 4 " b [3:0] $end
$var wire 1 # cin $end
$var wire 4 $ sum [3:0] $end
$var wire 1 % cout $end
$upscope $end
$upscope $end
$enddefinitions $end

#0
b0001 !
b0010 "
0#
$dumpvars
b0011 $
0%
$end

#10000
b0111 !
b1001 "
1#

#10100
b0000 $
1%
```

Header section: timescale, scope hierarchy, variable declarations, identifiers (printable ASCII compaction).
Body: timestamps `#t`, scalar values `0!` or `1!`, vector values `b1010 $`, real values `r1.5 $`.

## Concepts

### Identifier compaction

Each variable gets a unique short identifier from the printable ASCII range (`!` to `~`, 94 chars; expand to multi-char for >94 vars). For 1000 signals, identifiers are 2 chars on average; saves ~10× on file size compared to full names.

### Timestamp coalescing

Multiple value changes at the same time share one `#t` line:
```
#1000
0!
1"
b101 #
```

### Initial dump

The `$dumpvars` block at t=0 records every signal's initial value. Required by VCD spec; tools that index VCD use it as a baseline.

### Scope hierarchy

VCD scopes mirror module hierarchy. `$scope module top` opens; `$upscope $end` closes. Signals declared between are scoped to that module. The viewer's signal browser tree comes from this hierarchy.

## Public API

```python
from dataclasses import dataclass
from pathlib import Path


@dataclass
class VcdWriter:
    path: Path
    timescale: str = "1ps"
    
    def open(self) -> None: ...
    def close(self) -> None: ...
    
    def scope(self, kind: str, name: str) -> "ScopeContext":
        """Returns a context manager that opens and closes a scope."""
        ...
    
    def declare(self, kind: str, width: int, name: str) -> str:
        """Declare a variable; returns its compact ID."""
        ...
    
    def end_definitions(self) -> None: ...
    
    def time(self, t: int) -> None:
        """Emit a #t timestamp."""
        ...
    
    def value(self, var_id: str, value: object) -> None:
        """Emit a value change for a variable."""
        ...
    
    def dump_initial(self, values: dict[str, object]) -> None: ...


# Subscribe interface (used by hardware-vm)

def attach_vcd(vm: "HardwareVM", writer: VcdWriter) -> None:
    """Wire up the VM to feed value-change events into the writer."""
    ...
```

## Worked Example — 4-bit Adder

```python
from hardware_vm import HardwareVM
from vcd_writer import VcdWriter, attach_vcd

vm = HardwareVM(hir=adder_hir)
writer = VcdWriter(Path("adder.vcd"), timescale="1ps")
writer.open()
attach_vcd(vm, writer)
vm.run(until_time=20_000)   # 20 ns
writer.close()

# adder.vcd is ~3 KB; opens in GTKWave
```

The `attach_vcd` helper:
1. Opens scopes mirroring HIR's module hierarchy.
2. Declares one VCD variable per HIR Net.
3. Subscribes to `value_change` events from VM.
4. On each event, writes timestamp (if needed) and the value change.

## Edge Cases

| Scenario | Handling |
|---|---|
| Net with X or Z | VCD scalar `x!` or `z!`; vector `b10x1 #`. |
| Real-valued signal | `r1.500000 #`. Uses `$var real ...`. |
| Signal renamed during sim | Not supported by VCD; warn at HIR level. |
| Multiple events at same time, same signal | VCD spec is fine with this; value-change uses last write. |
| Hierarchy aliasing (same signal under multiple names) | VCD allows multi-name vars; we emit single name + alias as a separate `$var`. |
| File exceeds disk capacity | Streaming write fails gracefully; error at next emit. |
| Negative time (some VCDs allow) | We forbid; VM time is non-negative. |
| Time wrap (64-bit overflow) | VCD uses decimal integers; no overflow concern at picosecond timescale (2.3 quintillion ps available). |
| Special chars in scope or variable names | Escape per VCD spec or warn. |

## Test Strategy

### Unit (95%+)
- Header generation: timescale, scope, var, end.
- Identifier compaction: 1, 94, 95, 1000 signals all generate valid IDs.
- Time: timestamps are non-decreasing.
- Value encoding: bit, vector, real.

### Integration
- Run the 4-bit adder; produced VCD opens cleanly in GTKWave.
- Diff the VCD against a hand-crafted reference: bit-identical (modulo header date).
- ARM1 simulator output: VCD reads cleanly; signals match reference.

## Conformance

| Standard | Coverage |
|---|---|
| **IEEE 1364-2005 §18** (VCD) | Full subset: scope, var (wire/reg/integer/real), value changes, $dumpvars, $end |
| **Extended VCD (EVCD)** | Out of scope for v1; future work |
| **FST** (binary, 50× smaller, GTKWave-native) | Out of scope for v1; future spec |
| **LXT2** | Out of scope |

## Open Questions

1. Should we also write FST for large designs? Future spec `fst-writer.md`.
2. How to handle hierarchical signal aliasing? VCD allows; we emit aliases.

## Future Work

- FST writer (50× smaller binary format).
- LXT2 writer.
- Streaming VCD reader (for diff/equivalence checking).
- VCD compression (gzip on close).
