# hardware-vm

Event-driven simulator for HIR. Drives a design with stimulus, runs continuous assignments, emits value-change events.

See [`code/specs/hardware-vm.md`](../../../specs/hardware-vm.md) for the design.

## Quick start

```python
from hdl_elaboration import elaborate_verilog
from hardware_vm import HardwareVM, Event

src = """
module adder4(input [3:0] a, input [3:0] b, input cin,
              output [3:0] sum, output cout);
  assign {cout, sum} = a + b + cin;
endmodule
"""

hir = elaborate_verilog(src, top="adder4")
vm = HardwareVM(hir)

# Subscribe to value changes (vcd-writer hooks here)
events = []
vm.subscribe(lambda e: events.append(e))

# Drive inputs
vm.set_input("a", 5)
vm.set_input("b", 3)
vm.set_input("cin", 0)

# Read outputs
assert vm.read("sum") == 8
assert vm.read("cout") == 0

# Carry-out case
vm.set_input("a", 0xF)
vm.set_input("b", 0x1)
assert vm.read("sum") == 0
assert vm.read("cout") == 1
```

## v0.1.0 scope

Combinational continuous assignments only. The 4-bit adder smoke test passes end-to-end:

- HIR ContAssign with binary ops, slice, concat
- Input port driving via `set_input`
- Output port reading via `read`
- Sensitivity inference: when an input changes, dependent ContAssigns re-evaluate
- Subscriber callbacks receive `Event(time, signal, new_value, old_value)` for every value change
- Force/release for testbench overrides

## Out of scope (v0.2.0)

- Behavioral processes (`always @(...)`, `initial`, `process`)
- `wait` / `@` / `#delay` suspensions
- 9-state StdLogic resolution
- Clocks and posedge/negedge sensitivity
- Delta cycles past delta=0 (currently bottoming out at 0 because no NBAs)

## Testing

```bash
pytest tests/                  # all tests
ruff check src/                # lint
```

MIT license.
