# IAS machine (1951) functional simulator

This crate models the stored-program computer built by the Institute for
Advanced Study. It implements the source-documented 40-bit complement
arithmetic, 4,096-word architectural address space, paired 20-bit orders,
accumulator and arithmetic register, buffered right-order control, and all 21
numbered operations plus stop.

The functional model deliberately stops at instruction boundaries. Williams
memory timing, valves, gates, and physical control pulses belong to the paired
gate-level package.

```rust
use ias_simulator::{encode_instruction, pack_instructions, IasSimulator, Opcode, HALT};

let mut machine = IasSimulator::new();
let program = [pack_instructions(
    encode_instruction(Opcode::Load, 1)?,
    HALT,
)?];
machine.load_words(&program, 0)?;
machine.write_memory(1, 7)?;
let result = machine.run(2)?;

assert_eq!(result.final_state.accumulator, 7);
# Ok::<(), ias_simulator::IasError>(())
```

The normative architecture and lifecycle contract is
[`../../../specs/07ac-ias-simulator.md`](../../../specs/07ac-ias-simulator.md).

## Development

```bash
bash BUILD
```
