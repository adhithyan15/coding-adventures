# EDSAC (1949) functional simulator

This crate implements the 1949 EDSAC order code from the University of
Cambridge's surviving documentation and reference simulator. It models the
1024 short-location store, short and long operands, sequence control, order
tank, 71-bit accumulator, multiplier and multiplicand, paper-tape input, and
teleprinter state.

The model is intentionally instruction-level. Mercury delay-line timing and
valve electronics belong to the paired gate-level package.

```rust
use edsac_simulator::{encode_order, EdsacSimulator, Length, Opcode};

let mut machine = EdsacSimulator::new();
let program = [
    encode_order(Opcode::A, 10, Length::Short),
    encode_order(Opcode::Z, 0, Length::Short),
];
machine.load_words(&program, 0)?;
machine.write_memory(10, 1)?;
let result = machine.run(10)?;

assert!(result.halted);
# Ok::<(), edsac_simulator::EdsacError>(())
```

The normative architecture and lifecycle contract is
[`../../../specs/07ab-edsac-simulator.md`](../../../specs/07ab-edsac-simulator.md).

## Development

```bash
bash BUILD
```
