# vcd-writer

Streaming VCD (Value Change Dump) waveform writer per IEEE 1364 §18. Output is consumable by GTKWave, surfer, ModelSim, and every other waveform viewer.

Decoupled from any specific simulator: emit value changes via `writer.value_change(time, var_id, value)`. An attach helper for `hardware-vm` (or any callback-style event emitter) is provided.

See [`code/specs/vcd-writer.md`](../../../specs/vcd-writer.md).

## Quick start

```python
from vcd_writer import VcdWriter

with VcdWriter("trace.vcd", timescale="1ps") as w:
    w.open_scope("top", "module")
    a_id   = w.declare("a", 4)
    b_id   = w.declare("b", 4)
    sum_id = w.declare("sum", 4)
    w.close_scope()
    w.end_definitions()

    w.dump_initial({a_id: 0, b_id: 0, sum_id: 0})

    w.value_change(10, a_id, 5)
    w.value_change(10, b_id, 3)
    w.value_change(10, sum_id, 8)

    w.value_change(20, a_id, 7)
    w.value_change(20, sum_id, 10)
```

## Wiring up to hardware-vm

```python
from hardware_vm import HardwareVM
from vcd_writer import VcdWriter, attach_to_callback_emitter

vm = HardwareVM(hir)

with VcdWriter("trace.vcd") as w:
    w.open_scope("top", "module")
    name_to_id = {
        "a":    w.declare("a", 4),
        "b":    w.declare("b", 4),
        "sum":  w.declare("sum", 4),
        "cout": w.declare("cout", 1),
    }
    w.close_scope()
    w.end_definitions()
    w.dump_initial({})

    vm.subscribe(attach_to_callback_emitter(w, name_to_var_id=name_to_id))

    vm.set_input("a", 5)
    vm.set_input("b", 3)
    # trace.vcd now contains the value-change events
```

## v0.1.0 scope

- Full IEEE 1364 §18 VCD subset: `$date`, `$version`, `$timescale`, `$scope`, `$var`, `$enddefinitions`, `$dumpvars`, `#t` time markers, scalar + vector value changes.
- Identifier compaction (printable ASCII, base-94 → 1 char for first 94 vars, 2 for the next ~9000).
- 4-state values: `0`, `1`, `x`, `z` for scalars; binary string for vectors.
- Real values via `r<value>` form.
- Coverage 95%+ on the implementation.

MIT.
