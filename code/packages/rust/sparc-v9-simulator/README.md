# SPARC V9 simulator (Rust)

A checked, deterministic implementation of the bounded SPARC V9 problem-state
integer contract in [Spec 07aw](../../../specs/07aw-sparc-v9-simulator.md).
It models 64-bit integer execution, the normal global register set, four
overlapping register windows, integer condition codes, PC/nPC delayed control
flow, big-endian memory, and typed transactional lifecycle behavior.

The normative source is SPARC International's *The SPARC Architecture Manual,
Version 9* (SAV09R1459912) and its published errata. Floating point, VIS,
privileged/trap state, alternate spaces and global sets, MMU/TLB behavior,
atomics, caches, buses, peripherals, timing, and cycle accuracy are explicitly
outside this first cell.

```rust
use sparc_v9_simulator::{assemble, encoding, SparcV9Simulator};

let program = assemble(&[
    encoding::encode_add_imm(1, 0, 42),
    encoding::encode_ta(0),
]);
let mut cpu = SparcV9Simulator::new();
let result = cpu.execute(&program, 0x100, 2)?;
assert_eq!(cpu.read_register(1)?, 42);
assert!(result.halted);
# Ok::<(), sparc_v9_simulator::SparcV9Error>(())
```

Every successful step exposes exact pre-execution instruction bytes, PC/nPC
before and after, uppercase mnemonic, delay/annul metadata, and complete state.
Failed steps and failed bounded runs are atomic. Verification includes opcode
classification boundaries, integer families, all condition senses, register
windows, memory widths/pairs, delay-slot chaining and annulment, lifecycle
faults, strict lint/docs gates, and measured line coverage.

The completion baseline is 91.22% line coverage (1,216/1,333).
