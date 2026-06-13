# hardware-vm

Event-driven combinational simulation kernel for the silicon-stack HIR.

## What it does

`hardware-vm` evaluates **continuous assignments** (`ContAssign`) reactively: when
an input changes, the VM re-evaluates every assignment sensitive to that signal and
cascades until quiescence.  This models the behaviour of purely combinational RTL
(wires, assigns, no clocked registers).

### Conceptual model

```
assign sum = a + b
```

1. Build a **sensitivity map**: which signals appear on each `ContAssign` RHS?
2. At t=0, evaluate every assignment once (bootstrap).
3. On `set_input("a", 3)`: update `a`, look up all assignments sensitive to `a`,
   re-evaluate them (updating `sum`), and cascade any further changes.

## Where it fits

```
hdl-ir → hardware-vm → vcd-writer
                    → subscriber callbacks (coverage, assertion, ...)
```

`hardware-vm` sits between the HIR structural description and observable waveforms.

## Usage

```rust
use hardware_vm::HardwareVm;
use hdl_ir::Hir;

let hir = Hir::from_json(json_str).unwrap();
let mut vm = HardwareVm::new(hir).unwrap();

// Drive inputs.
vm.set_input("a", 3).unwrap();
vm.set_input("b", 5).unwrap();

// Read any signal (input or output).
assert_eq!(vm.read("sum"), 8);

// Subscribe to all value-change events.
vm.subscribe(|ev| println!("t={} {} = {}", ev.time, ev.signal, ev.new_value));

// Force / release for debug override.
vm.force("sum", 99);
vm.release("sum"); // reverts to combinational value
```

## API

| Method | Description |
|--------|-------------|
| `new(hir)` | Build VM from an HIR document; bootstraps all assigns at t=0 |
| `set_input(name, val)` | Drive an `In` or `Inout` port; cascades reactive updates |
| `read(name)` | Read current value of any signal |
| `force(name, val)` | Override a signal (ignores normal drivers) |
| `release(name)` | Remove force; normal drivers resume |
| `subscribe(cb)` | Register a `Fn(&Event)` called on every value change |
| `stats()` | Return event count, cont-assign run count, final time |

## Supported expression types

`Lit`, `NetRef`, `VarRef`, `PortRef`, `Slice`, `Concat`, `Replication`,
`Unary`, `Binary`, `Ternary`.  `FunCall`, `SystemCall`, and `Attr` return 0
(v0.2.0 work item).

## Tests

```
cargo test -p hardware-vm
```

11 integration tests + 1 doc-test.
