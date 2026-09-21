# DEC PDP-8/E functional simulator

This crate models the base 4K-word PDP-8/E processor described by DEC's 1972
*Small Computer Handbook*. It implements the complete six memory-reference
instruction families, zero/current-page and indirect/auto-index addressing,
Group 1 and Group 2 OPR microinstructions, CPU-control IOTs, interrupt entry,
and the front-panel switch register.

External device IOTs are surfaced as structured trace events. That keeps the
CPU deterministic while providing the bus boundary needed for later Teletype,
storage, and system-level simulation. The optional KE8-E Extended Arithmetic
Element is not silently approximated: Group 3 instructions return a typed,
atomic `UnsupportedEae` error.

Programs use little-endian pairs containing one 12-bit word. Values with any
of the upper four bits set are rejected. Direct word loading is also available.

```rust
use pdp8_simulator::{encode_memory_reference, Opcode, Pdp8Simulator, HLT};

let mut cpu = Pdp8Simulator::new();
let program = [
    encode_memory_reference(Opcode::Tad, false, false, 0o20),
    HLT,
];
cpu.load_words(&program, 0)?;
cpu.write_memory(0o20, 7)?;
let result = cpu.run(10)?;

assert_eq!(result.final_state.ac, 7);
# Ok::<(), pdp8_simulator::Pdp8Error>(())
```

The architecture contract is in
[`../../../specs/07aa-pdp8-simulator.md`](../../../specs/07aa-pdp8-simulator.md).

## Development

```bash
bash BUILD
```
