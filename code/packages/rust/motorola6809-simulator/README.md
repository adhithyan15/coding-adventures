# Motorola MC6809 simulator

This crate is a checked instruction-level simulator for Motorola's original
MC6809. It owns the complete 64 KiB address space and the A, B/D, X, Y, U, S,
PC, DP, and CC registers. It implements every documented base, page-2, and
page-3 opcode plus all legal indexed-addressing postbytes.

Programs are raw MC6809 bytes and multi-byte values are big-endian. Execution
is deterministic and bounded: `step` is atomic, `run` and `execute` are
transactional, and retained traces are capped before allocation. `SYNC` and
`CWAI` provide deterministic wait-state boundaries for CPU-only programs,
while software interrupts retain their architectural stack and vector effects.

```rust
use motorola6809_simulator::Motorola6809Simulator;

let mut cpu = Motorola6809Simulator::new();
cpu.load(&[0x86, 0x15, 0xc6, 0x03, 0x3d, 0x13], 0)?;
let result = cpu.run(8)?;

assert_eq!(result.final_state.d(), 0x003f);
assert!(result.halted);
# Ok::<(), motorola6809_simulator::Motorola6809Error>(())
```

The normative architecture and acceptance contract is
[`../../../specs/07ah-motorola-6809-simulator.md`](../../../specs/07ah-motorola-6809-simulator.md).

## Development

```bash
bash BUILD
```

