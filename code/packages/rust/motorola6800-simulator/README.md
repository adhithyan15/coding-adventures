# Motorola MC6800 functional simulator

This crate is a checked behavioral model of Motorola's original 1974 MC6800,
the 8-bit processor that preceded the unrelated 16/32-bit Motorola 68000. It
implements all 72 base operations and all 198 documented opcode encodings from
Motorola's 1976 programming reference.

The simulator exposes complete owned state: A and B, X, SP, PC, condition
codes, all 64 KiB of memory, wait state, and the exact installed-program range.
Its seven addressing modes, big-endian words, descending stack, software
interrupt frame, and condition-code rules follow the original manuals.

```rust
use motorola6800_simulator::Motorola6800Simulator;

// LDAA #42 ; WAI
let mut cpu = Motorola6800Simulator::new();
let result = cpu.execute(&[0x86, 42, 0x3e], 0x1000, 8)?;

assert!(result.halted);
assert_eq!(result.final_state.a, 42);
# Ok::<(), motorola6800_simulator::Motorola6800Error>(())
```

`WAI` is the deterministic CPU-only halt boundary. It still creates the
documented status stack, leaving later interrupt pins and peripherals for the
system-level model. `SWI` and `RTI` retain their architectural behavior.

All fetch, load, restoration, and bounded-execution failures are typed and
atomic. Successful steps include exact raw bytes, width, mnemonic, PCs, and a
complete post-instruction snapshot.

The architecture contract is in
[`../../../specs/07ae-motorola-6800-simulator.md`](../../../specs/07ae-motorola-6800-simulator.md).

## Development

```bash
bash BUILD
```
